//! Qualification tool server: `snapshot_read` over a directory snapshot with
//! the result shape `lifecycle-service` returns; task tools are recorded only.

use crate::driver::ToolServer;
use crate::spec;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub const MAX_RESULT: u64 = 65536;

pub struct SnapshotServer {
    pub id: String,
    members: BTreeMap<String, Vec<u8>>,
    pub published: Vec<Value>,
    pub questions: Vec<Value>,
}

impl SnapshotServer {
    /// Load every regular file under `root` (relative paths as member ids).
    pub fn load(id: &str, root: &Path) -> Result<Self, String> {
        let mut members = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let ty = entry.file_type().map_err(|e| e.to_string())?;
                let path = entry.path();
                if ty.is_dir() {
                    stack.push(path);
                } else if ty.is_file() {
                    let rel = path
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .into_owned();
                    members.insert(rel, std::fs::read(&path).map_err(|e| e.to_string())?);
                }
            }
        }
        if members.is_empty() || members.len() > 4096 {
            return Err("snapshot must have 1..=4096 members".into());
        }
        Ok(Self {
            id: id.into(),
            members,
            published: Vec::new(),
            questions: Vec::new(),
        })
    }

    pub fn manifest(&self) -> Vec<Value> {
        self.members
            .iter()
            .map(|(k, v)| json!({"member_id":k,"bytes":v.len(),"sha256":spec::sha256_hex(v)}))
            .collect()
    }

    fn read(&self, args: &Value) -> Result<String, String> {
        let snapshot = args["snapshot_id"].as_str().ok_or("snapshot_id")?;
        let member = args["member_id"].as_str().ok_or("member_id")?;
        let offset = args["offset"].as_u64().ok_or("offset")?;
        let length = args["length"].as_u64().ok_or("length")?;
        if snapshot != self.id {
            return Err("unknown snapshot".into());
        }
        let bytes = self.members.get(member).ok_or("unknown member")?;
        let end = offset.checked_add(length).ok_or("range")?;
        if length == 0 || length > MAX_RESULT || end > bytes.len() as u64 {
            return Err("range outside the member".into());
        }
        let slice = &bytes[offset as usize..end as usize];
        let sha = spec::sha256_hex(slice);
        let value = match std::str::from_utf8(slice) {
            Ok(text) => json!({"encoding":"utf8","text":text,"sha256":sha,
                "snapshot_id":snapshot,"member_id":member,"offset":offset,"length":length}),
            Err(_) => json!({"encoding":"hex","bytes_hex":spec::hex(slice),"sha256":sha,
                "snapshot_id":snapshot,"member_id":member,"offset":offset,"length":length}),
        };
        Ok(value.to_string())
    }
}

impl ToolServer for SnapshotServer {
    fn call(&mut self, tool: &str, arguments: &Value) -> (String, bool) {
        match tool {
            "snapshot_read" => match self.read(arguments) {
                Ok(text) => (text, false),
                Err(e) => (format!("snapshot_read refused: {e}"), true),
            },
            "result_publish" => {
                self.published.push(arguments.clone());
                (
                    json!({"published":true,"result_id":arguments["result_id"]}).to_string(),
                    false,
                )
            }
            "question_open" => {
                self.questions.push(arguments.clone());
                (
                    json!({"opened":true,"question_id":arguments["question_id"]}).to_string(),
                    false,
                )
            }
            _ => ("unknown tool".into(), true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_in_bounds_and_refuses_outside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs/a.md"), "hello world").unwrap();
        let mut s = SnapshotServer::load("snap-1", dir.path()).unwrap();
        let (text, err) = s.call(
            "snapshot_read",
            &json!({"snapshot_id":"snap-1","member_id":"docs/a.md","offset":6,"length":5}),
        );
        assert!(!err);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["text"], "world");
        assert_eq!(v["sha256"], spec::sha256_hex(b"world"));
        let (_, err) = s.call(
            "snapshot_read",
            &json!({"snapshot_id":"snap-1","member_id":"docs/a.md","offset":6,"length":6}),
        );
        assert!(err);
        let (_, err) = s.call(
            "snapshot_read",
            &json!({"snapshot_id":"other","member_id":"docs/a.md","offset":0,"length":1}),
        );
        assert!(err);
        let (_, err) = s.call("shell", &json!({}));
        assert!(err);
    }
}
