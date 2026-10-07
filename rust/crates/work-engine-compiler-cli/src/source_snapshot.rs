use crate::protocol::ProtocolError;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use work_engine_compiler::sha256_hex;

const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PATHS: usize = 256;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

pub struct Snapshots {
    files: HashMap<String, Vec<u8>>,
    total: usize,
}
impl Snapshots {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            total: 0,
        }
    }
    pub fn capture(&mut self, path: &Path) -> Result<&[u8], ProtocolError> {
        let key = path
            .to_str()
            .ok_or_else(|| ProtocolError::source("source_unavailable", "non-UTF-8 source path"))?
            .to_owned();
        if !self.files.contains_key(&key) {
            if self.files.len() >= MAX_PATHS {
                return Err(ProtocolError::source(
                    "resource_limit",
                    "too many distinct source paths",
                ));
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
                .open(path)
                .map_err(|issue| {
                    ProtocolError::source(
                        "source_unavailable",
                        format!("source open failed: {issue}"),
                    )
                })?;
            let before = file.metadata().map_err(|issue| {
                ProtocolError::source(
                    "source_unavailable",
                    format!("source metadata failed: {issue}"),
                )
            })?;
            if !before.is_file() {
                return Err(ProtocolError::source(
                    "source_unavailable",
                    "source must be a regular file",
                ));
            }
            if before.len() > MAX_FILE_BYTES as u64 {
                return Err(ProtocolError::source(
                    "resource_limit",
                    "source exceeds per-file limit",
                ));
            }
            let mut bytes = Vec::with_capacity(before.len() as usize);
            file.by_ref()
                .take((MAX_FILE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|issue| {
                    ProtocolError::source(
                        "source_unavailable",
                        format!("source read failed: {issue}"),
                    )
                })?;
            if bytes.len() > MAX_FILE_BYTES
                || self.total.saturating_add(bytes.len()) > MAX_TOTAL_BYTES
            {
                return Err(ProtocolError::source(
                    "resource_limit",
                    "captured source exceeds limit",
                ));
            }
            let after = file.metadata().map_err(|issue| {
                ProtocolError::source(
                    "source_unavailable",
                    format!("source metadata failed: {issue}"),
                )
            })?;
            use std::os::unix::fs::MetadataExt;
            if before.dev() != after.dev()
                || before.ino() != after.ino()
                || before.len() != after.len()
                || before.mtime() != after.mtime()
                || before.mtime_nsec() != after.mtime_nsec()
                || bytes.len() as u64 != after.len()
            {
                return Err(ProtocolError::source(
                    "source_mismatch",
                    "source changed during capture",
                ));
            }
            self.total += bytes.len();
            self.files.insert(key.clone(), bytes);
        }
        Ok(&self.files[&key])
    }
    pub fn into_files(self) -> HashMap<String, Vec<u8>> {
        self.files
    }
}

/// POSIX lexical resolution compatible with Node path.resolve for ordinary absolute paths.
pub fn resolve(root: &Path, authored: &str) -> PathBuf {
    let input = Path::new(authored);
    let combined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    let mut result = PathBuf::from("/");
    for part in combined.components() {
        match part {
            Component::RootDir => result = PathBuf::from("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(item) => result.push(item),
            Component::Prefix(_) => {}
        }
    }
    result
}
pub fn evidence(files: &HashMap<String, Vec<u8>>) -> serde_json::Value {
    let mut paths: Vec<_> = files
        .iter()
        .map(|(path, bytes)| serde_json::json!({"path":path,"sha256":sha256_hex(bytes)}))
        .collect();
    paths.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    serde_json::Value::Array(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn separate_lexical_symlinks_keep_distinct_capture_identities_across_retarget() {
        let temp = TempDir::new().unwrap();
        let one = temp.path().join("one");
        let two = temp.path().join("two");
        std::fs::write(&one, b"first").unwrap();
        std::fs::write(&two, b"second").unwrap();
        let alias_a = temp.path().join("alias-a");
        let alias_b = temp.path().join("alias-b");
        symlink(&one, &alias_a).unwrap();
        symlink(&one, &alias_b).unwrap();
        let mut snapshots = Snapshots::new();
        assert_eq!(snapshots.capture(&alias_a).unwrap(), b"first");
        std::fs::remove_file(&alias_b).unwrap();
        symlink(&two, &alias_b).unwrap();
        assert_eq!(snapshots.capture(&alias_b).unwrap(), b"second");
        std::fs::write(&one, b"changed after capture").unwrap();
        assert_eq!(snapshots.capture(&alias_a).unwrap(), b"first");
        assert_eq!(snapshots.into_files().len(), 2);
    }
}
