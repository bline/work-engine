use crate::skill::parse_yaml;
use crate::{
    CompiledSkill, CompilerError, ErrorCode, canonical_json, compile_skill_unverified, sha256_hex,
};
use serde_json::{Value, json};
use std::collections::HashMap;

fn mismatch(path: &str, message: &str) -> CompilerError {
    CompilerError::new(ErrorCode::SourceMismatch, Some(path.to_owned()), message)
}
fn aeg_error(message: &str) -> CompilerError {
    CompilerError::new(ErrorCode::AegProtocol, Some("/aeg".to_owned()), message)
}
fn required<'a>(
    files: &'a HashMap<String, Vec<u8>>,
    path: &str,
) -> Result<&'a [u8], CompilerError> {
    files
        .get(path)
        .map(Vec::as_slice)
        .ok_or_else(|| mismatch(path, "captured source is missing"))
}
fn matching(
    files: &HashMap<String, Vec<u8>>,
    path: &str,
    expected: &str,
) -> Result<(), CompilerError> {
    if sha256_hex(required(files, path)?) != expected {
        return Err(mismatch(path, "captured source digest mismatch"));
    }
    Ok(())
}

/// Schema-validated input. It carries no filesystem or process observation.
#[derive(Debug)]
pub struct PreparedSkill {
    structure: Value,
    compiled: CompiledSkill,
}
/// Pure comparison against supplied bytes. It does not attest the origin of those bytes.
#[derive(Debug)]
pub struct SourceCheckedSkill {
    structure: Value,
    compiled: CompiledSkill,
    pinned_projection: Option<Value>,
}
/// Pure comparison against a supplied projection. It does not attest a Python invocation.
#[derive(Debug)]
pub struct ComparedSkill {
    compiled: CompiledSkill,
}
impl ComparedSkill {
    pub fn into_compiled(self) -> CompiledSkill {
        self.compiled
    }
}

pub fn prepare_skill_verified(
    structure: &[u8],
    interface: &[u8],
) -> Result<PreparedSkill, CompilerError> {
    let compiled = compile_skill_unverified(structure, interface)?;
    let structure = parse_yaml(structure, "/structure")?;
    Ok(PreparedSkill {
        structure,
        compiled,
    })
}
impl PreparedSkill {
    pub fn paths(&self) -> Vec<String> {
        let mut paths = vec![
            self.structure["source"]["path"]
                .as_str()
                .unwrap()
                .to_owned(),
        ];
        for binding in self.structure["source_bindings"].as_array().unwrap() {
            paths.push(binding["path"].as_str().unwrap().to_owned());
        }
        if let Some(role) = self
            .structure
            .get("role_profile")
            .filter(|value| !value.is_null())
        {
            paths.push(role["projection"]["path"].as_str().unwrap().to_owned());
        }
        paths
    }
    pub fn check_sources(
        self,
        files: &HashMap<String, Vec<u8>>,
    ) -> Result<SourceCheckedSkill, CompilerError> {
        let source = &self.structure["source"];
        let source_path = source["path"].as_str().unwrap();
        matching(files, source_path, source["sha256"].as_str().unwrap())?;
        for binding in self.structure["source_bindings"].as_array().unwrap() {
            matching(
                files,
                binding["path"].as_str().unwrap(),
                binding["sha256"].as_str().unwrap(),
            )?;
        }
        let bytes = required(files, source_path)?;
        let frontmatter = format!(
            "---\nname: {}\ndescription: {}\n---\n\n",
            self.structure["frontmatter"]["name"].as_str().unwrap(),
            self.structure["frontmatter"]["description"]
                .as_str()
                .unwrap()
        );
        if !bytes.starts_with(frontmatter.as_bytes()) {
            return Err(mismatch(
                source_path,
                "canonical frontmatter prefix differs",
            ));
        }
        let mut previous = frontmatter.len();
        for section in self.structure["sections"].as_array().unwrap() {
            let span = &section["source_span"];
            let start = span["start_byte"]
                .as_u64()
                .ok_or_else(|| mismatch(source_path, "invalid section offset"))?
                as usize;
            let end = span["end_byte"]
                .as_u64()
                .ok_or_else(|| mismatch(source_path, "invalid section offset"))?
                as usize;
            if start != previous {
                return Err(mismatch(
                    source_path,
                    "section spans are not contiguous from frontmatter",
                ));
            }
            let rendered = format!(
                "{} {}\n{}",
                "#".repeat(section["heading"]["level"].as_u64().unwrap() as usize),
                section["heading"]["text"].as_str().unwrap(),
                section["content"].as_str().unwrap()
            );
            if bytes.get(start..end) != Some(rendered.as_bytes()) {
                return Err(mismatch(
                    source_path,
                    "section bytes differ from canonical source",
                ));
            }
            previous = end;
        }
        if previous != bytes.len() {
            return Err(mismatch(
                source_path,
                "section spans do not reach source EOF",
            ));
        }
        let pinned_projection = if let Some(role) = self
            .structure
            .get("role_profile")
            .filter(|value| !value.is_null())
        {
            let path = role["projection"]["path"].as_str().unwrap();
            matching(files, path, role["projection"]["sha256"].as_str().unwrap())?;
            Some(parse_yaml(required(files, path)?, "/role_projection")?)
        } else {
            None
        };
        Ok(SourceCheckedSkill {
            structure: self.structure,
            compiled: self.compiled,
            pinned_projection,
        })
    }
}
impl SourceCheckedSkill {
    pub fn role_request(&self) -> Option<Value> {
        let role = self.compiled.ir.get("role_profile")?.as_object()?;
        let mut candidate = json!({"label":role["label"], "objective":role["objective"], "context_lifetime":role["context_lifetime"]});
        for (key, value) in role["relations"].as_object().unwrap() {
            candidate[key] = value.clone();
        }
        Some(json!({"schema_version":1,"role_id":role["role_id"],"role":candidate}))
    }
    pub fn compare_projection(
        mut self,
        aeg: Option<&Value>,
        invariant_sha256: Option<&str>,
        environment_sha256: Option<&str>,
        script_sha256: Option<&str>,
    ) -> Result<ComparedSkill, CompilerError> {
        if let Some(projection) = self.pinned_projection {
            let projection_path = self.structure["role_profile"]["projection"]["path"]
                .as_str()
                .unwrap();
            let envelope =
                aeg.ok_or_else(|| aeg_error("role requires observed Python projection"))?;
            let obj = envelope
                .as_object()
                .ok_or_else(|| aeg_error("Python envelope must be an object"))?;
            let expected = [
                "schema_version",
                "status",
                "backend",
                "backend_sha256",
                "canonical_role_match",
                "projection",
            ];
            if obj.len() != expected.len()
                || obj.keys().any(|key| !expected.contains(&key.as_str()))
            {
                return Err(aeg_error("Python envelope has invalid fields"));
            }
            if envelope["schema_version"] != 1
                || envelope["status"] != "closed_projection"
                || envelope["backend"] != "work-engine.agent-environment-graph.v1"
                || envelope["canonical_role_match"] != true
            {
                return Err(aeg_error("Python envelope identity or status differs"));
            }
            if envelope["backend_sha256"].as_str() != script_sha256 {
                return Err(aeg_error("Python backend digest differs"));
            }
            if envelope["projection"]["schema_version"] != 1
                || envelope["projection"]["status"] != "generated_projection"
            {
                return Err(aeg_error("Python projection contract differs"));
            }
            let inputs = &envelope["projection"]["source_inputs"];
            if inputs["invariant_catalog"]["path"] != "../workflow-invariants.md"
                || inputs["invariant_catalog"]["sha256"].as_str() != invariant_sha256
                || inputs["role_environment"]["path"] != "../agent-environments.yaml"
                || inputs["role_environment"]["sha256"].as_str() != environment_sha256
            {
                return Err(aeg_error(
                    "Python source hashes differ from captured inputs",
                ));
            }
            if canonical_json(&envelope["projection"]) != canonical_json(&projection) {
                return Err(mismatch(
                    projection_path,
                    "complete role projection differs from pinned generated oracle",
                ));
            }
            self.compiled.ir["role_projection"] = json!({"backend":envelope["backend"],"backend_sha256":envelope["backend_sha256"],"canonical_role_match":true,"projection":envelope["projection"]});
        } else if aeg.is_some() {
            return Err(aeg_error(
                "role-free skill must not carry Python observation",
            ));
        }
        Ok(ComparedSkill {
            compiled: self.compiled,
        })
    }
}
