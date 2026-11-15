//! Campaign-local prelaunch filesystem identity. This is actor-attributed observation, not
//! atomic resolution or kernel/descendant execution attestation.
use crate::base::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ObjectBinding {
    #[serde(rename = "file")]
    File { sha256: String },
    #[serde(rename = "symlink")]
    Symlink { sha256: String, link_target: String },
}
impl ObjectBinding {
    pub fn sha(&self) -> &str {
        match self {
            Self::File { sha256 } | Self::Symlink { sha256, .. } => sha256,
        }
    }
    pub fn observe(path: &Path) -> Result<Self, String> {
        let m = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if m.file_type().is_symlink() {
            let t = fs::read_link(path).map_err(|e| e.to_string())?;
            let text = t
                .to_str()
                .ok_or("non-UTF8 link target unsupported")?
                .to_owned();
            Ok(Self::Symlink {
                sha256: sha256_hex(t.as_os_str().as_bytes()),
                link_target: text,
            })
        } else if m.is_file() {
            Ok(Self::File {
                sha256: sha256_hex(&fs::read(path).map_err(|e| e.to_string())?),
            })
        } else {
            Err(format!(
                "{}: required object is neither regular file nor link",
                path.display()
            ))
        }
    }
    pub fn custody(&self, path: &str) -> Value {
        match self {
            Self::File { sha256 } => json!({"path":path,"type":"file","sha256":sha256}),
            Self::Symlink {
                sha256,
                link_target,
            } => json!({"path":path,"type":"symlink","sha256":sha256,"link_target":link_target}),
        }
    }
    pub fn from_custody(v: &Value) -> Result<Self, String> {
        let sha256 = v["sha256"]
            .as_str()
            .ok_or("input sha256 missing")?
            .to_owned();
        match v["type"].as_str() {
            Some("file") if v.get("link_target").is_none() => Ok(Self::File { sha256 }),
            Some("symlink") => Ok(Self::Symlink {
                sha256,
                link_target: v["link_target"]
                    .as_str()
                    .ok_or("link target missing")?
                    .to_owned(),
            }),
            _ => Err("contradictory or unsupported custody input kind".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub id: String,
    /// Top-level argv/context is map-owned; descendant declarations additionally bind source.
    pub source: Option<String>,
    pub argv: Vec<String>,
    pub cwd: String,
    pub path_env: Option<String>,
    pub lookup: String,
    pub links: Vec<String>,
    pub endpoint: String,
}

/// Resolve every relevant component, including directory symlinks. Bound link traversals to 40
/// (revisiting a link is not necessarily a cycle); refuse dangling, unsupported and cyclic paths.
pub fn resolve(path: &Path) -> Result<(Vec<String>, String), String> {
    if !path.is_absolute() {
        return Err("resolution path must be absolute".into());
    }
    let mut todo: VecDeque<_> = path
        .components()
        .map(|c| c.as_os_str().to_owned())
        .collect();
    let mut current = PathBuf::new();
    let mut links = Vec::new();
    while let Some(c) = todo.pop_front() {
        match Path::new(&c).components().next() {
            Some(Component::RootDir) => {
                current = PathBuf::from("/");
                continue;
            }
            Some(Component::CurDir) => continue,
            Some(Component::ParentDir) => {
                current.pop();
                continue;
            }
            Some(Component::Normal(_)) => current.push(c),
            _ => return Err("unsupported path component".into()),
        }
        let m =
            fs::symlink_metadata(&current).map_err(|e| format!("{}: {e}", current.display()))?;
        if m.file_type().is_symlink() {
            if links.len() == 40 {
                return Err("cyclic or excessive link chain".into());
            }
            links.push(current.to_str().ok_or("non-UTF8 path")?.to_owned());
            let target = fs::read_link(&current).map_err(|e| e.to_string())?;
            current.pop();
            for c in target.components().rev() {
                todo.push_front(c.as_os_str().to_owned());
            }
        } else if !todo.is_empty() && !m.is_dir() {
            return Err("non-directory path component".into());
        }
    }
    if !matches!(
        ObjectBinding::observe(&current)?,
        ObjectBinding::File { .. }
    ) {
        return Err("endpoint must be regular".into());
    }
    Ok((
        links,
        current.to_str().ok_or("non-UTF8 endpoint")?.to_owned(),
    ))
}

impl Invocation {
    pub fn validate(
        &self,
        objects: &BTreeMap<String, ObjectBinding>,
        captured: Option<&BTreeMap<String, ObjectBinding>>,
    ) -> Result<(), String> {
        if self.id.is_empty()
            || self.argv.is_empty()
            || self.argv[0].is_empty()
            || !Path::new(&self.cwd).is_absolute()
        {
            return Err("incomplete invocation identity/context".into());
        }
        let command = &self.argv[0];
        let lookup = if command.contains('/') {
            let p = Path::new(command);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                Path::new(&self.cwd).join(p)
            }
        } else {
            let path = self
                .path_env
                .as_ref()
                .ok_or("bare command requires explicit PATH")?;
            let mut found = None;
            for d in path.split(':') {
                let dir = if d.is_empty() {
                    PathBuf::from(&self.cwd)
                } else if Path::new(d).is_absolute() {
                    PathBuf::from(d)
                } else {
                    Path::new(&self.cwd).join(d)
                };
                let p = dir.join(command);
                if fs::metadata(&p)
                    .map(|m| {
                        use std::os::unix::fs::PermissionsExt;
                        m.is_file() && m.permissions().mode() & 0o111 != 0
                    })
                    .unwrap_or(false)
                {
                    found = Some(p);
                    break;
                }
            }
            found.ok_or("command unresolved in declared PATH")?
        };
        if lookup.to_str() != Some(&self.lookup) {
            return Err("invocation lookup contradicts argv/context".into());
        }
        let (links, endpoint) = resolve(&lookup)?;
        if links != self.links || endpoint != self.endpoint {
            return Err("declared resolution differs from current link chain/endpoint".into());
        }
        for p in self.links.iter().chain(std::iter::once(&self.endpoint)) {
            let expected = objects
                .get(p)
                .ok_or_else(|| format!("resolution object absent from pins: {p}"))?;
            let now = ObjectBinding::observe(Path::new(p))?;
            if &now != expected {
                return Err(format!("resolution object changed: {p}"));
            }
            if let Some(c) = captured {
                if c.get(p) != Some(expected) {
                    return Err(format!(
                        "resolution object missing/contradictory in custody: {p}"
                    ));
                }
            }
        }
        if !matches!(
            objects.get(&self.endpoint),
            Some(ObjectBinding::File { .. })
        ) {
            return Err("endpoint lacks regular content binding".into());
        }
        if self
            .links
            .iter()
            .any(|p| !matches!(objects.get(p), Some(ObjectBinding::Symlink { .. })))
        {
            return Err("chain object lacks link binding".into());
        }
        Ok(())
    }
}
