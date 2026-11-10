//! One immutable snapshot of a register tree at an exact commit.

use crate::error::{BadRequest, CannotRead, Error, NoRegister};
use crate::objects::{split_tree_path, EntryKind, GitObjects};
use gix::hash::ObjectId;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const SNAPSHOT_SCHEMA: &str = "research_register_snapshot_v1";
const MAX_TREE_DEPTH: usize = 32;

/// One entry under the register tree. Files carry their verified bytes; links and gitlinks are
/// recorded by id only and never read through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    kind: EntryKind,
    id: ObjectId,
    bytes: Option<Arc<[u8]>>,
    sha256: Option<String>,
}

impl SnapshotEntry {
    pub fn kind(&self) -> EntryKind {
        self.kind
    }
    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
    pub fn size(&self) -> Option<u64> {
        self.bytes.as_ref().map(|b| b.len() as u64)
    }
    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }
}

/// The register tree at one commit, read once through [`GitObjects`]. It keeps the repository
/// handle so that later checks read from the same opened repository.
pub struct PinnedSnapshot {
    objects: GitObjects,
    requested: String,
    commit: ObjectId,
    root_tree: ObjectId,
    register_path: String,
    register_tree: ObjectId,
    entries: BTreeMap<String, SnapshotEntry>,
}

impl PinnedSnapshot {
    pub fn build(objects: GitObjects, rev: &str, register_path: &str) -> Result<Self, Error> {
        split_tree_path(register_path)?;
        let commit = objects.resolve(rev)?;
        let root_tree = objects.commit(commit)?.tree;
        let register_tree = match objects.entry_at(commit, register_path)? {
            None => return Err(NoRegister::Absent { path: register_path.into() }.into()),
            Some(e) if e.kind != EntryKind::Tree => {
                return Err(NoRegister::NotATree { path: register_path.into(), kind: e.kind.as_str().into() }.into());
            }
            Some(e) => e.id,
        };
        let mut entries = BTreeMap::new();
        collect(&objects, register_tree, "", 0, &mut entries)?;
        Ok(PinnedSnapshot {
            objects,
            requested: rev.into(),
            commit,
            root_tree,
            register_path: register_path.into(),
            register_tree,
            entries,
        })
    }

    pub fn objects(&self) -> &GitObjects {
        &self.objects
    }
    pub fn requested(&self) -> &str {
        &self.requested
    }
    pub fn commit(&self) -> ObjectId {
        self.commit
    }
    pub fn root_tree(&self) -> ObjectId {
        self.root_tree
    }
    pub fn register_path(&self) -> &str {
        &self.register_path
    }
    pub fn register_tree(&self) -> ObjectId {
        self.register_tree
    }
    /// Entries keyed by path relative to the register tree, in byte order.
    pub fn entries(&self) -> &BTreeMap<String, SnapshotEntry> {
        &self.entries
    }
    pub fn entry(&self, path: &str) -> Option<&SnapshotEntry> {
        self.entries.get(path)
    }

    /// Provenance and per-entry identity, without file bytes. Field and entry order are fixed.
    pub fn identity(&self) -> SnapshotIdentity<'_> {
        SnapshotIdentity {
            schema: SNAPSHOT_SCHEMA,
            outcome: "ok",
            requested: &self.requested,
            commit: self.commit.to_string(),
            root_tree: self.root_tree.to_string(),
            register_path: &self.register_path,
            register_tree: self.register_tree.to_string(),
            entries: self
                .entries
                .iter()
                .map(|(path, e)| EntryIdentity {
                    path,
                    kind: e.kind.as_str(),
                    id: e.id.to_string(),
                    size: e.size(),
                    sha256: e.sha256.as_deref(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SnapshotIdentity<'a> {
    pub schema: &'static str,
    pub outcome: &'static str,
    pub requested: &'a str,
    pub commit: String,
    pub root_tree: String,
    pub register_path: &'a str,
    pub register_tree: String,
    pub entries: Vec<EntryIdentity<'a>>,
}

#[derive(Debug, Serialize)]
pub struct EntryIdentity<'a> {
    pub path: &'a str,
    pub kind: &'static str,
    pub id: String,
    pub size: Option<u64>,
    pub sha256: Option<&'a str>,
}

fn collect(
    objects: &GitObjects,
    tree: ObjectId,
    prefix: &str,
    depth: usize,
    out: &mut BTreeMap<String, SnapshotEntry>,
) -> Result<(), Error> {
    if depth > MAX_TREE_DEPTH {
        return Err(CannotRead::Malformed { id: tree.to_string(), reason: format!("tree nesting exceeds {MAX_TREE_DEPTH}") }.into());
    }
    for entry in objects.tree(tree)? {
        if entry.name.contains('/') {
            return Err(BadRequest::Path { path: entry.name, reason: "tree entry name contains '/'".into() }.into());
        }
        let path = if prefix.is_empty() { entry.name.clone() } else { format!("{prefix}/{}", entry.name) };
        let (bytes, sha256) = if entry.kind.is_file() {
            let bytes: Arc<[u8]> = objects.blob(entry.id)?.into();
            let digest = format!("{:x}", Sha256::digest(&bytes));
            (Some(bytes), Some(digest))
        } else {
            (None, None)
        };
        out.insert(path.clone(), SnapshotEntry { kind: entry.kind, id: entry.id, bytes, sha256 });
        if entry.kind == EntryKind::Tree {
            collect(objects, entry.id, &path, depth + 1, out)?;
        }
    }
    Ok(())
}
