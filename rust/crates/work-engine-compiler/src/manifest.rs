//! Pure runtime-manifest projection and requirement satisfaction. Transport records
//! are host assertions, not source-verification or capability authorities.
use crate::{canonical_json, sha256_hex};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub code: &'static str,
    pub path: Option<String>,
    pub message: String,
}
impl ManifestError {
    fn new(
        code: &'static str,
        path: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
    fn invalid(path: &str, message: impl Into<String>) -> Self {
        Self::new("invalid_manifest", Some(path.to_owned()), message)
    }
    fn requirement(path: &str, message: impl Into<String>) -> Self {
        Self::new("invalid_requirements", Some(path.to_owned()), message)
    }
    fn unsatisfied(message: impl Into<String>) -> Self {
        Self::new("requirements_unsatisfied", None, message)
    }
}
fn record<'a>(v: &'a Value, path: &str) -> Result<&'a Map<String, Value>, ManifestError> {
    v.as_object()
        .ok_or_else(|| ManifestError::invalid(path, "must be an object"))
}
fn text<'a>(v: Option<&'a Value>, path: &str) -> Result<&'a str, ManifestError> {
    v.and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ManifestError::invalid(path, "must be a non-empty string"))
}
fn ident(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn allowed(map: &Map<String, Value>, names: &[&str], path: &str) -> Result<(), ManifestError> {
    if let Some(k) = map.keys().find(|k| !names.contains(&k.as_str())) {
        return Err(ManifestError::invalid(
            path,
            format!("unsupported field {k}"),
        ));
    }
    Ok(())
}
fn resolve(base: &str, raw: &str) -> String {
    let joined = if raw.starts_with('/') {
        raw.to_owned()
    } else {
        format!("{base}/{raw}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for p in joined.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(p),
        }
    }
    format!("/{}", parts.join("/"))
}
fn values(v: Option<&Value>, label: &str) -> Result<Vec<String>, ManifestError> {
    let Some(v) = v else { return Ok(vec![]) };
    if v.is_null() {
        return Ok(vec![]);
    }
    let a = v
        .as_array()
        .ok_or_else(|| ManifestError::invalid(label, "must be an array"))?;
    let mut out = Vec::new();
    for x in a {
        let s = x
            .as_str()
            .ok_or_else(|| ManifestError::invalid(label, "item must be text"))?;
        if !ident(s) || out.contains(&s.to_owned()) {
            return Err(ManifestError::invalid(
                label,
                "items must be unique identifiers",
            ));
        }
        out.push(s.to_owned());
    }
    out.sort();
    Ok(out)
}
fn skills(v: Option<&Value>, base: &str, role: &str) -> Result<Vec<Value>, ManifestError> {
    let a = v.and_then(Value::as_array).ok_or_else(|| {
        ManifestError::invalid(&format!("/roles/{role}/skills"), "must be an array")
    })?;
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for (i, s) in a.iter().enumerate() {
        let p = format!("/roles/{role}/skills/{i}");
        let m = record(s, &p)?;
        allowed(
            m,
            &[
                "name",
                "path",
                "compiled_environment",
                "compiled_skill_sha256",
                "capabilities",
                "effects",
            ],
            &p,
        )?;
        let name = text(m.get("name"), &format!("{p}/name"))?;
        let raw = text(m.get("path"), &format!("{p}/path"))?;
        if seen.contains(&name) {
            return Err(ManifestError::invalid(&p, "duplicate skill name"));
        }
        seen.push(name);
        let mut o = json!({"name":name,"path":resolve(base,raw)});
        if let Some(c) = m.get("compiled_environment").filter(|v| !v.is_null()) {
            let cm = record(c, &format!("{p}/compiled_environment"))?;
            allowed(
                cm,
                &["structure", "interface"],
                &format!("{p}/compiled_environment"),
            )?;
            let st = text(
                cm.get("structure"),
                &format!("{p}/compiled_environment/structure"),
            )?;
            let it = text(
                cm.get("interface"),
                &format!("{p}/compiled_environment/interface"),
            )?;
            let fingerprint = text(
                m.get("compiled_skill_sha256"),
                &format!("{p}/compiled_skill_sha256"),
            )?;
            if !sha(fingerprint) {
                return Err(ManifestError::invalid(&p, "invalid compiled fingerprint"));
            }
            o["compiledEnvironment"] =
                json!({"structure":resolve(base,st),"interface":resolve(base,it)});
            o["compiledSkillSha256"] = json!(fingerprint);
            o["capabilities"] = json!(values(m.get("capabilities"), &format!("{p}/capabilities"))?);
            o["effects"] = json!(values(m.get("effects"), &format!("{p}/effects"))?);
        } else if m.get("compiled_skill_sha256").is_some_and(|v| !v.is_null()) {
            return Err(ManifestError::invalid(
                &p,
                "fingerprint requires compiled_environment",
            ));
        }
        out.push(o);
    }
    Ok(out)
}
fn thread_options(v: Option<&Value>, base: &str, role: &str) -> Result<Value, ManifestError> {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(json!({}));
    };
    let p = format!("/roles/{role}/thread_options");
    let m = record(v, &p)?;
    allowed(
        m,
        &[
            "cwd",
            "approval_policy",
            "sandbox",
            "model",
            "reasoning_effort",
            "service_tier",
            "personality",
        ],
        &p,
    )?;
    let mut o = Map::new();
    for (authored, runtime) in [
        ("cwd", "cwd"),
        ("approval_policy", "approvalPolicy"),
        ("sandbox", "sandbox"),
        ("model", "model"),
        ("reasoning_effort", "effort"),
        ("service_tier", "serviceTier"),
        ("personality", "personality"),
    ] {
        if let Some(v) = m.get(authored) {
            let s = text(Some(v), &format!("{p}/{authored}"))?;
            let permitted = match authored {
                "approval_policy" => Some(&["untrusted", "on-request", "never"][..]),
                "sandbox" => Some(&["read-only", "workspace-write", "danger-full-access"][..]),
                "personality" => Some(&["none", "friendly", "pragmatic"][..]),
                _ => None,
            };
            if permitted.is_some_and(|a| !a.contains(&s)) {
                return Err(ManifestError::invalid(
                    &p,
                    format!("unsupported {authored}"),
                ));
            }
            o.insert(
                runtime.to_owned(),
                json!(if authored == "cwd" {
                    resolve(base, s)
                } else {
                    s.to_owned()
                }),
            );
        }
    }
    Ok(Value::Object(o))
}
fn option<'a>(options: &'a Map<String, Value>, key: &str) -> Result<&'a str, ManifestError> {
    options
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| s.starts_with('/'))
        .ok_or_else(|| ManifestError::invalid(&format!("/options/{key}"), "must be absolute text"))
}
pub fn project_runtime_manifest(document: &Value, options: &Value) -> Result<Value, ManifestError> {
    let d = record(document, "/document")?;
    allowed(d, &["schema_version", "manifest_id", "roles"], "/document")?;
    if d.get("schema_version") != Some(&json!(1)) {
        return Err(ManifestError::invalid(
            "/document/schema_version",
            "must be 1",
        ));
    }
    let id = text(d.get("manifest_id"), "/document/manifest_id")?;
    if !ident(id) {
        return Err(ManifestError::invalid(
            "/document/manifest_id",
            "invalid identifier",
        ));
    }
    let role_input = record(d.get("roles").unwrap_or(&Value::Null), "/document/roles")?;
    if role_input.is_empty() {
        return Err(ManifestError::invalid(
            "/document/roles",
            "must define a role",
        ));
    }
    let o = record(options, "/options")?;
    allowed(
        o,
        &[
            "base_directory",
            "identity_base_directory",
            "requirements_base_directory",
            "source_path",
            "source_sha256",
            "runtime_requirements_by_role",
        ],
        "/options",
    )?;
    let base = option(o, "base_directory")?;
    let identity_base = option(o, "identity_base_directory")?;
    let requirements_base = option(o, "requirements_base_directory")?;
    let requirements = record(
        o.get("runtime_requirements_by_role")
            .unwrap_or(&Value::Null),
        "/options/runtime_requirements_by_role",
    )?;
    let mut roles = Map::new();
    for (role_id, role) in role_input {
        if !ident(role_id) {
            return Err(ManifestError::invalid(
                "/document/roles",
                "invalid role identifier",
            ));
        }
        let p = format!("/document/roles/{role_id}");
        let r = record(role, &p)?;
        allowed(
            r,
            &[
                "contract",
                "compiled_environment",
                "compiled_skill_sha256",
                "developer_instructions",
                "thread_options",
                "skills",
                "capabilities",
                "effects",
                "continuity",
            ],
            &p,
        )?;
        let contract = resolve(
            identity_base,
            text(r.get("contract"), &format!("{p}/contract"))?,
        );
        let ident_skills = skills(r.get("skills"), identity_base, role_id)?;
        let delivery_skills = skills(r.get("skills"), base, role_id)?;
        let index = ident_skills
            .iter()
            .position(|s| s["path"] == contract)
            .ok_or_else(|| {
                ManifestError::invalid(&p, "contract must be present in exact skill inputs")
            })?;
        let fingerprint = match r.get("compiled_skill_sha256").filter(|v| !v.is_null()) {
            Some(v) => {
                let s = text(Some(v), &format!("{p}/compiled_skill_sha256"))?;
                if !sha(s) {
                    return Err(ManifestError::invalid(&p, "invalid compiled fingerprint"));
                }
                json!(s)
            }
            None => Value::Null,
        };
        let instructions = match r.get("developer_instructions").filter(|v| !v.is_null()) {
            Some(v) => text(Some(v), &format!("{p}/developer_instructions"))?.to_owned(),
            None => String::new(),
        };
        let continuity = match r.get("continuity").filter(|v| !v.is_null()) {
            Some(v) => v.as_str().unwrap_or(""),
            None => "ephemeral",
        };
        if !["ephemeral", "retained"].contains(&continuity) {
            return Err(ManifestError::invalid(&p, "unsupported continuity"));
        }
        let mut projected_skills = Vec::new();
        for (i, s) in delivery_skills.iter().enumerate() {
            let mut s = s.clone();
            s["identityPath"] = ident_skills[i]["path"].clone();
            let key = format!("{role_id}:{}", s["name"].as_str().unwrap());
            if let Some(req) = requirements.get(&key).filter(|v| !v.is_null()) {
                s["runtimeRequirements"] = req.clone()
            }
            projected_skills.push(s)
        }
        let role_req = requirements
            .get(role_id)
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(Value::Null);
        let contract_identity = json!({"path":contract});
        let mut template = json!({"roleContract":{"path":contract,"activatedPath":delivery_skills[index]["path"]},"developerInstructions":instructions,"threadOptions":thread_options(r.get("thread_options"),identity_base,role_id)?,"skills":projected_skills,"capabilities":values(r.get("capabilities"),&format!("{p}/capabilities"))?,"effects":values(r.get("effects"),&format!("{p}/effects"))?,"continuity":continuity,"compiledSkillSha256":fingerprint,"runtimeRequirements":role_req});
        let mut identity = template.clone();
        identity["roleContract"] = contract_identity;
        identity["skills"] = json!(ident_skills);
        template["runtimeEnvironmentRevision"] =
            json!(sha256_hex(canonical_json(&identity).as_bytes()));
        roles.insert(role_id.clone(), template);
    }
    let source_sha = match o.get("source_sha256").filter(|v| !v.is_null()) {
        Some(v) => text(Some(v), "/options/source_sha256")?.to_owned(),
        None => sha256_hex(canonical_json(document).as_bytes()),
    };
    let source_path = match o.get("source_path").filter(|v| !v.is_null()) {
        Some(v) => json!(resolve("/", text(Some(v), "/options/source_path")?)),
        None => Value::Null,
    };
    Ok(
        json!({"manifestId":id,"source":{"manifestId":id,"schemaVersion":1,"path":source_path,"sha256":source_sha},"roles":roles,"requirementsBaseDirectory":resolve("/",requirements_base)}),
    )
}
fn validate_normalized_manifest(m: &Map<String, Value>) -> Result<(), ManifestError> {
    let exact =
        |map: &Map<String, Value>, names: &[&str], path: &str| -> Result<(), ManifestError> {
            if map.len() != names.len()
                || map.keys().any(|k| !names.contains(&k.as_str()))
                || names.iter().any(|k| !map.contains_key(*k))
            {
                return Err(ManifestError::invalid(
                    path,
                    "normalized fields differ from protocol",
                ));
            }
            Ok(())
        };
    exact(
        m,
        &["manifestId", "source", "roles", "requirementsBaseDirectory"],
        "/manifest",
    )?;
    let id = text(m.get("manifestId"), "/manifest/manifestId")?;
    if !ident(id) {
        return Err(ManifestError::invalid(
            "/manifest/manifestId",
            "invalid identifier",
        ));
    }
    let source = record(&m["source"], "/manifest/source")?;
    exact(
        source,
        &["manifestId", "schemaVersion", "path", "sha256"],
        "/manifest/source",
    )?;
    if source["manifestId"] != id
        || source["schemaVersion"] != 1
        || !source["path"].is_null() && !source["path"].as_str().is_some_and(|s| s.starts_with('/'))
        || !source["sha256"].as_str().is_some_and(sha)
    {
        return Err(ManifestError::invalid(
            "/manifest/source",
            "invalid source identity",
        ));
    }
    if !m["requirementsBaseDirectory"]
        .as_str()
        .is_some_and(|s| s.starts_with('/'))
    {
        return Err(ManifestError::invalid(
            "/manifest/requirementsBaseDirectory",
            "must be absolute",
        ));
    }
    let roles = record(&m["roles"], "/manifest/roles")?;
    if roles.is_empty() {
        return Err(ManifestError::invalid(
            "/manifest/roles",
            "must define a role",
        ));
    }
    for (id, rv) in roles {
        if !ident(id) {
            return Err(ManifestError::invalid("/manifest/roles", "invalid role ID"));
        }
        let p = format!("/manifest/roles/{id}");
        let r = record(rv, &p)?;
        exact(
            r,
            &[
                "roleContract",
                "developerInstructions",
                "threadOptions",
                "skills",
                "capabilities",
                "effects",
                "continuity",
                "compiledSkillSha256",
                "runtimeRequirements",
                "runtimeEnvironmentRevision",
            ],
            &p,
        )?;
        let c = record(&r["roleContract"], &format!("{p}/roleContract"))?;
        exact(c, &["path", "activatedPath"], &format!("{p}/roleContract"))?;
        for name in ["path", "activatedPath"] {
            if !c[name].as_str().is_some_and(|s| s.starts_with('/')) {
                return Err(ManifestError::invalid(
                    &p,
                    "contract paths must be absolute",
                ));
            }
        }
        if !r["developerInstructions"].is_string()
            || !r["threadOptions"].is_object()
            || !r["runtimeEnvironmentRevision"].as_str().is_some_and(sha)
            || !r["compiledSkillSha256"].is_null()
                && !r["compiledSkillSha256"].as_str().is_some_and(sha)
            || !["ephemeral", "retained"].contains(&r["continuity"].as_str().unwrap_or(""))
        {
            return Err(ManifestError::invalid(&p, "invalid role scalar"));
        }
        for field in ["capabilities", "effects"] {
            values(r.get(field), &format!("{p}/{field}"))?;
            if !r[field].is_array() {
                return Err(ManifestError::invalid(&p, "grant array missing"));
            }
        }
        let a = r["skills"]
            .as_array()
            .ok_or_else(|| ManifestError::invalid(&p, "skills must be an array"))?;
        let mut names = Vec::new();
        for skill in a {
            let sm = record(skill, &format!("{p}/skills"))?;
            let name = text(sm.get("name"), &format!("{p}/skills/name"))?;
            if names.contains(&name) {
                return Err(ManifestError::invalid(&p, "duplicate skill name"));
            }
            names.push(name);
            for field in ["path", "identityPath"] {
                if !sm
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.starts_with('/'))
                {
                    return Err(ManifestError::invalid(&p, "skill paths must be absolute"));
                }
            }
            if let Some(fp) = sm.get("compiledSkillSha256")
                && !fp.as_str().is_some_and(sha)
            {
                return Err(ManifestError::invalid(&p, "invalid skill fingerprint"));
            }
        }
    }
    Ok(())
}
fn array_strings<'a>(v: Option<&'a Value>, path: &str) -> Result<Vec<&'a str>, ManifestError> {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(vec![]);
    };
    v.as_array()
        .ok_or_else(|| ManifestError::requirement(path, "must be an array"))?
        .iter()
        .map(|x| {
            x.as_str()
                .ok_or_else(|| ManifestError::requirement(path, "must contain strings"))
        })
        .collect()
}
pub fn satisfy_runtime_requirements(
    manifest: &Value,
    role_id: &str,
    requirements: &Value,
    skill_name: Option<&str>,
) -> Result<Value, ManifestError> {
    let m = record(manifest, "/manifest")?;
    validate_normalized_manifest(m)?;
    if !ident(role_id) {
        return Err(ManifestError::invalid("/role_id", "invalid role ID"));
    }
    let roles = record(&m["roles"], "/manifest/roles")?;
    let role = roles.get(role_id).ok_or_else(|| {
        ManifestError::unsatisfied(format!("runtime manifest does not define role {role_id}"))
    })?;
    let role = record(role, "/manifest/roles/role")?;
    let req = record(requirements, "/requirements")
        .map_err(|_| ManifestError::requirement("/requirements", "must be an object"))?;
    if req.get("schema_version") != Some(&json!(1)) {
        return Err(ManifestError::requirement(
            "/requirements/schema_version",
            "must be 1",
        ));
    }
    if req.get("verified_sources") != Some(&json!(true)) {
        return Err(ManifestError::unsatisfied(
            "runtime requirements are not bound to verified sources",
        ));
    }
    let digest = req
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| ManifestError::requirement("/requirements/sha256", "must be text"))?;
    let fingerprint = req
        .get("compiled_skill_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ManifestError::requirement("/requirements/compiled_skill_sha256", "must be text")
        })?;
    let mut unsigned = req.clone();
    unsigned.remove("sha256");
    if sha256_hex(canonical_json(&Value::Object(unsigned)).as_bytes()) != digest {
        return Err(ManifestError::unsatisfied(
            "runtime requirements digest mismatch",
        ));
    }
    let selected = if let Some(name) = skill_name {
        role.get("skills")
            .and_then(Value::as_array)
            .and_then(|a| a.iter().find(|s| s.get("name") == Some(&json!(name))))
            .ok_or_else(|| {
                ManifestError::unsatisfied(format!(
                    "runtime role {role_id} omits compiled skill {name}"
                ))
            })?
    } else {
        role.get("compiledSkillSha256")
            .ok_or_else(|| ManifestError::invalid("/manifest/roles", "missing fingerprint"))?;
        role.get("roleContract")
            .ok_or_else(|| ManifestError::invalid("/manifest/roles", "missing contract"))?;
        &Value::Null
    };
    let grants = if skill_name.is_some() {
        selected.get("capabilities")
    } else {
        role.get("capabilities")
    };
    let effects = if skill_name.is_some() {
        selected.get("effects")
    } else {
        role.get("effects")
    };
    let caps = array_strings(grants, "/manifest/capabilities")
        .map_err(|_| ManifestError::invalid("/manifest/capabilities", "invalid grants"))?;
    let eff = array_strings(effects, "/manifest/effects")
        .map_err(|_| ManifestError::invalid("/manifest/effects", "invalid grants"))?;
    if let Some(name) = skill_name {
        let role_caps = array_strings(role.get("capabilities"), "/manifest/role/capabilities")
            .map_err(|_| ManifestError::invalid("/manifest", "invalid grants"))?;
        for cap in &caps {
            if !role_caps.contains(cap) {
                return Err(ManifestError::unsatisfied(format!(
                    "compiled skill {name} capability {cap} is not granted to runtime role {role_id}"
                )));
            }
        }
    }
    for cap in array_strings(
        req.get("required_capabilities"),
        "/requirements/required_capabilities",
    )? {
        if !caps.contains(&cap) {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} is missing required capability {cap}"
            )));
        }
    }
    let ceiling = array_strings(
        req.get("capability_ceiling"),
        "/requirements/capability_ceiling",
    )?;
    for cap in caps {
        if !ceiling.contains(&cap) {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} exceeds capability ceiling with {cap}"
            )));
        }
    }
    let ceiling = array_strings(req.get("effect_ceiling"), "/requirements/effect_ceiling")?;
    let prohibited = array_strings(
        req.get("prohibited_effects"),
        "/requirements/prohibited_effects",
    )?;
    if let Some(name) = skill_name {
        let role_eff = array_strings(role.get("effects"), "/manifest/role/effects")
            .map_err(|_| ManifestError::invalid("/manifest", "invalid grants"))?;
        for effect in &eff {
            if !role_eff.contains(effect) {
                return Err(ManifestError::unsatisfied(format!(
                    "compiled skill {name} effect {effect} is not granted to runtime role {role_id}"
                )));
            }
        }
    }
    for effect in eff {
        if prohibited.contains(&effect) {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} grants prohibited effect {effect}"
            )));
        }
        if !ceiling.contains(&effect) {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} exceeds effect ceiling with {effect}"
            )));
        }
    }
    if skill_name.is_none() && role.get("continuity") != req.get("continuity") {
        return Err(ManifestError::unsatisfied(format!(
            "runtime role {role_id} has incompatible continuity"
        )));
    }
    let actual = if skill_name.is_some() {
        selected.get("compiledSkillSha256")
    } else {
        role.get("compiledSkillSha256")
    };
    if actual != Some(&json!(fingerprint)) {
        return Err(ManifestError::unsatisfied(format!(
            "runtime role {role_id} compiled skill fingerprint differs from requirements"
        )));
    }
    let contract = req
        .get("contract")
        .and_then(Value::as_object)
        .ok_or_else(|| ManifestError::requirement("/requirements/contract", "must be an object"))?;
    if contract.get("must_be_activated") != Some(&json!(true)) {
        return Err(ManifestError::requirement(
            "/requirements/contract/must_be_activated",
            "must require exact contract input",
        ));
    }
    let base = m["requirementsBaseDirectory"].as_str().ok_or_else(|| {
        ManifestError::invalid("/manifest/requirementsBaseDirectory", "must be text")
    })?;
    let required = resolve(
        base,
        contract
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ManifestError::requirement("/requirements/contract/path", "must be text")
            })?,
    );
    if skill_name.is_none() {
        if contract.get("kind") != Some(&json!("role_contract")) {
            return Err(ManifestError::unsatisfied(
                "role requirements must identify a role contract",
            ));
        }
        let rc = &role["roleContract"];
        if rc["path"] != required {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} contract differs from compiled requirements"
            )));
        }
        if !role["skills"]
            .as_array()
            .is_some_and(|a| a.iter().any(|s| s["path"] == rc["activatedPath"]))
        {
            return Err(ManifestError::unsatisfied(format!(
                "runtime role {role_id} omits the compiled contract input"
            )));
        }
    } else {
        if contract.get("kind") != Some(&json!("skill")) {
            return Err(ManifestError::unsatisfied(
                "compiled skill requirements must identify a secondary skill",
            ));
        }
        if selected["identityPath"] != required {
            return Err(ManifestError::unsatisfied(
                "compiled skill path differs from requirements",
            ));
        }
    }
    let mut receipt = json!({"schema_version":1,"role_id":role_id,"requirements_sha256":digest,"compiled_skill_sha256":fingerprint,"manifest_sha256":m["source"]["sha256"],"runtime_environment_revision":role["runtimeEnvironmentRevision"]});
    if let Some(name) = skill_name {
        receipt["skill_id"] = json!(name)
    }
    receipt["sha256"] = json!(sha256_hex(canonical_json(&receipt).as_bytes()));
    Ok(receipt)
}
