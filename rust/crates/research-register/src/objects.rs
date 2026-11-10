//! Verified Git object access over one repository opened in isolation.

use crate::error::{BadRequest, CannotRead, Error, Unsupported};
use gix::hash::{Kind as HashKind, ObjectId};
use gix::objs::Kind;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

const MAX_SYMBOLIC_DEPTH: usize = 8;
const MAX_TAG_DEPTH: usize = 8;

/// The kind of a tree entry, from the entry's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    Tree,
    Blob,
    BlobExecutable,
    Link,
    Gitlink,
}

impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntryKind::Tree => "tree",
            EntryKind::Blob => "blob",
            EntryKind::BlobExecutable => "blob_executable",
            EntryKind::Link => "link",
            EntryKind::Gitlink => "gitlink",
        }
    }
    /// Regular or executable file content. Links and gitlinks are never read through.
    pub fn is_file(self) -> bool {
        matches!(self, EntryKind::Blob | EntryKind::BlobExecutable)
    }
}

/// A tree's entries split by whether their names are UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeListing {
    pub entries: Vec<TreeEntry>,
    pub non_utf8_names: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub name: String,
    pub kind: EntryKind,
    pub id: ObjectId,
}

/// A verified commit: its tree and parents as recorded in the commit object itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub id: ObjectId,
    pub tree: ObjectId,
    pub parents: Vec<ObjectId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ancestry {
    Yes,
    No,
    /// The walk reached a commit that is not available, so `No` cannot be established.
    CannotEstablish,
}

/// Result of looking up an abbreviated object id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prefix {
    /// Exactly one object id in the store starts with the prefix. Reading it verifies it.
    Unique(ObjectId),
    Ambiguous,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobAt {
    Blob { id: ObjectId, bytes: Vec<u8> },
    NotABlob { id: ObjectId, kind: EntryKind },
    Absent,
}

/// Read-only access to one repository's objects. Every object returned has been checked against
/// the id and type requested.
pub struct GitObjects {
    repo: gix::Repository,
    path: String,
}

impl GitObjects {
    /// Open `path` (a working tree, linked worktree or git directory) in isolation from the
    /// environment and from user and system configuration. Refuses shallow repositories and the
    /// repository formats [`check_repository_format`] names.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let shown = path.display().to_string();
        check_repository_format(path)?;
        let options =
            gix::open::Options::isolated().config_overrides(["core.useReplaceRefs=false"]);
        let repo = gix::open_opts(path, options).map_err(|e| CannotRead::Open {
            path: shown.clone(),
            reason: e.to_string(),
        })?;
        if repo.object_hash() != HashKind::Sha1 {
            return Err(Unsupported::ObjectFormat {
                format: format!("{:?}", repo.object_hash()),
            }
            .into());
        }
        match repo.is_shallow() {
            Ok(false) => {}
            Ok(true) => return Err(Unsupported::ShallowRepository.into()),
            Err(e) => {
                return Err(CannotRead::ShallowState {
                    reason: e.to_string(),
                }
                .into());
            }
        }
        Ok(GitObjects { repo, path: shown })
    }

    /// The path the repository was opened from.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Read any object, checking that its content hashes to `id`.
    fn read(&self, id: ObjectId) -> Result<(Kind, Vec<u8>), Error> {
        let found = self
            .repo
            .try_find_object(id)
            .map_err(|e| CannotRead::ObjectStore {
                id: id.to_string(),
                reason: e.to_string(),
            })?;
        let object = found
            .ok_or_else(|| CannotRead::ObjectUnavailable { id: id.to_string() })?
            .detach();
        verify_hash(id, object.kind, &object.data)?;
        Ok((object.kind, object.data))
    }

    /// Read an object that must be of kind `expected`.
    fn read_kind(&self, id: ObjectId, expected: Kind) -> Result<Vec<u8>, Error> {
        let (kind, data) = self.read(id)?;
        verify_kind(id, expected, kind)?;
        Ok(data)
    }

    pub fn commit(&self, id: ObjectId) -> Result<Commit, Error> {
        let data = self.read_kind(id, Kind::Commit)?;
        Ok(parse_commit(id, &data)?)
    }

    /// The entries of tree `id`. A non-UTF-8 entry name is a malformed tree here; use
    /// [`GitObjects::tree_listing`] to see such names.
    pub fn tree(&self, id: ObjectId) -> Result<Vec<TreeEntry>, Error> {
        let listing = self.tree_listing(id)?;
        if !listing.non_utf8_names.is_empty() {
            return Err(CannotRead::Malformed {
                id: id.to_string(),
                reason: "entry name is not UTF-8".into(),
            }
            .into());
        }
        Ok(listing.entries)
    }

    /// The entries of tree `id` with UTF-8 names, plus the raw names that are not UTF-8 (A6: the
    /// register reports those as invalid content, R-01). A duplicate name (compared as bytes) or a
    /// name [`unsafe_entry_name`] refuses makes the tree malformed (`cannot_read`), checked on the
    /// raw bytes before any path is formed from them.
    pub fn tree_listing(&self, id: ObjectId) -> Result<TreeListing, Error> {
        let data = self.read_kind(id, Kind::Tree)?;
        let malformed = |reason: String| CannotRead::Malformed {
            id: id.to_string(),
            reason,
        };
        let mut entries = Vec::new();
        let mut non_utf8_names = Vec::new();
        let mut names: HashSet<Vec<u8>> = HashSet::new();
        for entry in gix::objs::TreeRefIter::from_bytes(&data, HashKind::Sha1) {
            let entry = entry.map_err(|e| malformed(format!("tree does not decode: {e}")))?;
            let kind = match entry.mode.kind() {
                gix::objs::tree::EntryKind::Tree => EntryKind::Tree,
                gix::objs::tree::EntryKind::Blob => EntryKind::Blob,
                gix::objs::tree::EntryKind::BlobExecutable => EntryKind::BlobExecutable,
                gix::objs::tree::EntryKind::Link => EntryKind::Link,
                gix::objs::tree::EntryKind::Commit => EntryKind::Gitlink,
            };
            let raw: &[u8] = entry.filename.as_ref();
            let shown = String::from_utf8_lossy(raw);
            // An unsafe or repeated name is a malformed tree (git fsck reports each); reading it
            // would make the snapshot and path lookups disagree, drop files or let one entry
            // stand for another.
            if let Some(why) = unsafe_entry_name(raw) {
                return Err(malformed(format!("entry name {shown:?} {why}")).into());
            }
            if !names.insert(raw.to_vec()) {
                return Err(malformed(format!("duplicate entry name {shown:?}")).into());
            }
            let Ok(name) = String::from_utf8(raw.to_vec()) else {
                non_utf8_names.push(raw.to_vec());
                continue;
            };
            entries.push(TreeEntry {
                name,
                kind,
                id: entry.oid.to_owned(),
            });
        }
        Ok(TreeListing {
            entries,
            non_utf8_names,
        })
    }

    pub fn blob(&self, id: ObjectId) -> Result<Vec<u8>, Error> {
        self.read_kind(id, Kind::Blob)
    }

    /// Resolve exactly: a full 40-hex id, `HEAD`, or a full reference name under `refs/`.
    /// Annotated tags are peeled through verified tag objects. Abbreviations are refused here;
    /// use [`GitObjects::resolve_prefix`].
    pub fn resolve(&self, rev: &str) -> Result<ObjectId, Error> {
        let bad = |reason: &str| BadRequest::Revision {
            rev: rev.into(),
            reason: reason.into(),
        };
        let start = if is_full_hex(rev) {
            ObjectId::from_hex(rev.as_bytes()).map_err(|e| bad(&e.to_string()))?
        } else if rev == "HEAD" || rev.starts_with("refs/") {
            self.reference_target(rev)?
        } else {
            return Err(bad(
                "expected a full 40-hex commit id, HEAD, or a full reference name under refs/",
            )
            .into());
        };
        let mut id = start;
        for _ in 0..=MAX_TAG_DEPTH {
            let (kind, data) = self.read(id)?;
            match kind {
                Kind::Commit => return Ok(id),
                Kind::Tag => id = parse_tag_target(id, &data)?,
                other => {
                    return Err(bad(&format!("names a {}, not a commit", kind_name(other))).into());
                }
            }
        }
        Err(CannotRead::Malformed {
            id: start.to_string(),
            reason: "tag chain too deep".into(),
        }
        .into())
    }

    fn reference_target(&self, name: &str) -> Result<ObjectId, Error> {
        if name != "HEAD"
            && let Err(e) = gix::validate::reference::name(name.as_bytes().into())
        {
            return Err(BadRequest::Revision {
                rev: name.into(),
                reason: format!("not a valid reference name: {e}"),
            }
            .into());
        }
        let mut current = name.to_string();
        for _ in 0..MAX_SYMBOLIC_DEPTH {
            let reference = match self.repo.try_find_reference(current.as_str()) {
                Ok(Some(r)) => r,
                Ok(None) => {
                    return Err(BadRequest::Revision {
                        rev: name.into(),
                        reason: format!("reference {current} does not exist"),
                    }
                    .into());
                }
                Err(e) => {
                    return Err(CannotRead::Reference {
                        name: current,
                        reason: e.to_string(),
                    }
                    .into());
                }
            };
            match &reference.inner.target {
                gix::refs::Target::Object(id) => return Ok(*id),
                gix::refs::Target::Symbolic(next) => current = next.as_bstr().to_string(),
            }
        }
        Err(CannotRead::Reference {
            name: name.into(),
            reason: "symbolic reference chain too deep".into(),
        }
        .into())
    }

    /// Look up an abbreviated id (4 to 40 lowercase hex digits).
    pub fn resolve_prefix(&self, hex: &str) -> Result<Prefix, Error> {
        if !(4..=40).contains(&hex.len())
            || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(BadRequest::Prefix {
                prefix: hex.into(),
                reason: "expected 4 to 40 lowercase hex digits".into(),
            }
            .into());
        }
        let prefix = gix::hash::Prefix::from_hex(hex).map_err(|e| BadRequest::Prefix {
            prefix: hex.into(),
            reason: e.to_string(),
        })?;
        let mut candidates = HashSet::new();
        if let Err(e) = self
            .repo
            .objects
            .lookup_prefix(prefix, Some(&mut candidates))
        {
            return Err(CannotRead::ObjectStore {
                id: hex.into(),
                reason: e.to_string(),
            }
            .into());
        }
        // A5 (owner decision D-A5): only verified objects of any type are candidates. Every id in
        // the store with the prefix is read and verified first; one that cannot be (a junk or
        // corrupt object file) makes resolution unreliable, so it is `cannot_read` even when
        // another candidate is valid, and it is never counted as an ambiguity candidate.
        let mut ids: Vec<ObjectId> = candidates.into_iter().collect();
        ids.sort_unstable();
        for id in &ids {
            self.read(*id)?;
        }
        Ok(match ids.as_slice() {
            [] => Prefix::Missing,
            [id] => Prefix::Unique(*id),
            _ => Prefix::Ambiguous,
        })
    }

    /// The entry at `path` in `commit`'s tree, or `None` when no such path exists.
    pub fn entry_at(&self, commit: ObjectId, path: &str) -> Result<Option<TreeEntry>, Error> {
        let components = split_tree_path(path)?;
        let mut tree = self.commit(commit)?.tree;
        let last = components.len() - 1;
        for (i, component) in components.iter().enumerate() {
            let entries = self.tree_listing(tree)?.entries;
            let Some(entry) = entries.into_iter().find(|e| e.name == *component) else {
                return Ok(None);
            };
            if i == last {
                return Ok(Some(entry));
            }
            if entry.kind != EntryKind::Tree {
                return Ok(None);
            }
            tree = entry.id;
        }
        Ok(None)
    }

    /// The verified file content at `path` in `commit`, if it is a file.
    pub fn blob_at(&self, commit: ObjectId, path: &str) -> Result<BlobAt, Error> {
        match self.entry_at(commit, path)? {
            None => Ok(BlobAt::Absent),
            Some(e) if e.kind.is_file() => Ok(BlobAt::Blob {
                id: e.id,
                bytes: self.blob(e.id)?,
            }),
            Some(e) => Ok(BlobAt::NotABlob {
                id: e.id,
                kind: e.kind,
            }),
        }
    }

    /// Whether `ancestor` is reachable from `descendant` through parents recorded in verified
    /// commit objects. Grafts, replace refs and commit-graph files are never consulted. `Yes`
    /// requires `ancestor` itself to be read as a verified commit, so an absent id gives
    /// `CannotEstablish` and a non-commit id gives `cannot_read`, never `Yes`.
    pub fn is_ancestor(&self, ancestor: ObjectId, descendant: ObjectId) -> Result<Ancestry, Error> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([descendant]);
        let mut incomplete = false;
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            match self.commit(id) {
                Ok(_) if id == ancestor => return Ok(Ancestry::Yes),
                Ok(commit) => queue.extend(commit.parents),
                Err(Error::CannotRead(CannotRead::ObjectUnavailable { .. })) => incomplete = true,
                Err(e) => return Err(e),
            }
        }
        Ok(if incomplete {
            Ancestry::CannotEstablish
        } else {
            Ancestry::No
        })
    }
}

/// Recompute the canonical object hash, `sha1("<type> <length>\0" + contents)`, and compare it
/// with the id the object was requested by.
pub(crate) fn verify_hash(requested: ObjectId, kind: Kind, data: &[u8]) -> Result<(), CannotRead> {
    let mut hasher = gix::hash::hasher(HashKind::Sha1);
    hasher.update(kind.as_bytes());
    hasher.update(format!(" {}\0", data.len()).as_bytes());
    hasher.update(data);
    let actual = hasher.try_finalize().map_err(|e| CannotRead::Malformed {
        id: requested.to_string(),
        reason: format!("hash computation failed: {e}"),
    })?;
    if actual != requested {
        return Err(CannotRead::Substituted {
            requested: requested.to_string(),
            content_hash: actual.to_string(),
        });
    }
    Ok(())
}

pub(crate) fn verify_kind(id: ObjectId, expected: Kind, actual: Kind) -> Result<(), CannotRead> {
    if expected != actual {
        return Err(CannotRead::TypeMismatch {
            id: id.to_string(),
            expected: kind_name(expected).into(),
            actual: kind_name(actual).into(),
        });
    }
    Ok(())
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Commit => "commit",
        Kind::Tree => "tree",
        Kind::Blob => "blob",
        Kind::Tag => "tag",
    }
}

fn is_full_hex(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn parse_hex_id(id: ObjectId, value: &[u8], what: &str) -> Result<ObjectId, CannotRead> {
    let malformed = || CannotRead::Malformed {
        id: id.to_string(),
        reason: format!("bad {what} id"),
    };
    let text = std::str::from_utf8(value).map_err(|_| malformed())?;
    if !is_full_hex(text) {
        return Err(malformed());
    }
    ObjectId::from_hex(value).map_err(|_| malformed())
}

/// Parse the tree and parent lines of a commit header, as Git does: `tree` first, then the
/// consecutive `parent` lines.
pub(crate) fn parse_commit(id: ObjectId, data: &[u8]) -> Result<Commit, CannotRead> {
    let mut lines = data.split(|b| *b == b'\n');
    let tree = match lines.next().and_then(|l| l.strip_prefix(b"tree ")) {
        Some(hex) => parse_hex_id(id, hex, "tree")?,
        None => {
            return Err(CannotRead::Malformed {
                id: id.to_string(),
                reason: "commit does not start with a tree line".into(),
            });
        }
    };
    let mut parents = Vec::new();
    for line in lines {
        match line.strip_prefix(b"parent ") {
            Some(hex) => parents.push(parse_hex_id(id, hex, "parent")?),
            None => break,
        }
    }
    Ok(Commit { id, tree, parents })
}

fn parse_tag_target(id: ObjectId, data: &[u8]) -> Result<ObjectId, CannotRead> {
    match data
        .split(|b| *b == b'\n')
        .next()
        .and_then(|l| l.strip_prefix(b"object "))
    {
        Some(hex) => parse_hex_id(id, hex, "tag object"),
        None => Err(CannotRead::Malformed {
            id: id.to_string(),
            reason: "tag does not start with an object line".into(),
        }),
    }
}

/// Why a tree entry name is refused, or `None` for a safe name. One rule on every platform: the
/// name is empty, `.`, `..`, contains `/`, or is equivalent to `.git` under the protections git
/// applies to tree entries (`git fsck`'s `hasDotgit`, which runs both regardless of platform):
///
/// - HFS+: `.git` compared case-insensitively after removing the code points HFS+ ignores
///   (U+200C-U+200F, U+202A-U+202E, U+206A-U+206F, U+FEFF), anywhere in the name;
/// - NTFS: in any segment between backslashes, `.git` or the short name `git~1`
///   (case-insensitive), followed only by dots and spaces up to the end or a `:` (alternate
///   data stream).
///
/// Both checks are gix-validate's ports of git's `is_hfs_dotgit` and `is_ntfs_dotgit`; the split
/// on backslash is git's own and is applied here because gix-validate leaves it to its Windows
/// option, which also refuses unrelated names. Checked against `git fsck` 2.53.0 (R-1b.4).
pub(crate) fn unsafe_entry_name(raw: &[u8]) -> Option<&'static str> {
    use gix::validate::path::component::{self, Error as Refused, Options};
    if raw.is_empty() {
        return Some("is empty");
    }
    if raw == b"." || raw == b".." {
        return Some("is '.' or '..'");
    }
    if raw.contains(&b'/') {
        return Some("contains '/'");
    }
    let dotgit = |name: &[u8], options: Options| {
        matches!(
            gix::validate::path::component(name.into(), None, options),
            Err(Refused::DotGitDir)
        )
    };
    let hfs = component::Options {
        protect_windows: false,
        protect_hfs: true,
        protect_ntfs: false,
    };
    let ntfs = component::Options {
        protect_windows: false,
        protect_hfs: false,
        protect_ntfs: true,
    };
    if dotgit(raw, hfs) || raw.split(|b| *b == b'\\').any(|s| dotgit(s, ntfs)) {
        return Some("is equivalent to '.git'");
    }
    None
}

/// Split a repository-relative tree path. Refuses empty, absolute, `.` and `..` components.
pub(crate) fn split_tree_path(path: &str) -> Result<Vec<&str>, BadRequest> {
    let bad = |reason: &str| BadRequest::Path {
        path: path.into(),
        reason: reason.into(),
    };
    if path.is_empty() || path.contains('\0') {
        return Err(bad("empty or contains NUL"));
    }
    let components: Vec<&str> = path.split('/').collect();
    if components
        .iter()
        .any(|c| c.is_empty() || *c == "." || *c == "..")
    {
        return Err(bad(
            "must be relative, without empty, '.' or '..' components",
        ));
    }
    Ok(components)
}

/// Extensions that do not change how objects or references are read here. At repository format
/// version 1 any other extension is refused, as git refuses an extension it does not know.
const KNOWN_EXTENSIONS: &[&str] = &[
    "objectformat",
    "refstorage",
    "worktreeconfig",
    "preciousobjects",
    "partialclone",
    "noop",
];

/// Check the repository's own configuration before gix opens it, so that an unsupported
/// repository is refused as `unsupported` with its reason rather than failing to open: a format
/// version above 1, an object format other than sha1, a reference backend other than files, or
/// (at version 1) an extension outside [`KNOWN_EXTENSIONS`]. The configuration is parsed with
/// gix-config's grammar, without includes. An unparseable configuration is `cannot_read`. The
/// object format is checked again after opening.
fn check_repository_format(path: &Path) -> Result<(), Error> {
    let Some(bytes) = repository_config_path(path).and_then(|p| std::fs::read(p).ok()) else {
        return Ok(());
    };
    let cannot = |reason: String| CannotRead::Open {
        path: path.display().to_string(),
        reason,
    };
    let config = parse_config(&bytes).map_err(cannot)?;
    let version = config
        .integer("core.repositoryformatversion")
        .map_err(|e| cannot(format!("core.repositoryformatversion: {e}")))?
        .unwrap_or(0);
    if !(0..=1).contains(&version) {
        return Err(Unsupported::FormatVersion { version }.into());
    }
    if let Some(format) = object_format(&config)
        && !format.eq_ignore_ascii_case("sha1")
    {
        return Err(Unsupported::ObjectFormat { format }.into());
    }
    if let Some(value) = config
        .string("extensions.refstorage")
        .map(|v| v.to_string())
        && !value.eq_ignore_ascii_case("files")
    {
        return Err(Unsupported::Extension {
            name: "refstorage".into(),
            value,
        }
        .into());
    }
    if version == 1 {
        for section in config.sections_by_name("extensions").into_iter().flatten() {
            for name in section.body().value_names() {
                let name = name.to_string();
                if !KNOWN_EXTENSIONS.contains(&name.to_ascii_lowercase().as_str()) {
                    let value = config
                        .string(format!("extensions.{name}").as_str())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    return Err(Unsupported::Extension { name, value }.into());
                }
            }
        }
    }
    Ok(())
}

/// The configuration file of the repository at `path`: in the common directory of a linked
/// worktree, otherwise in the git directory.
fn repository_config_path(path: &Path) -> Option<PathBuf> {
    let dot = path.join(".git");
    let git_dir: PathBuf = if dot.is_dir() {
        dot
    } else if dot.is_file() {
        let text = std::fs::read_to_string(&dot).ok()?;
        let target = PathBuf::from(text.strip_prefix("gitdir:")?.trim());
        if target.is_absolute() {
            target
        } else {
            path.join(target)
        }
    } else {
        path.to_path_buf()
    };
    let dir = match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(common) => {
            let common = PathBuf::from(common.trim());
            if common.is_absolute() {
                common
            } else {
                git_dir.join(common)
            }
        }
        Err(_) => git_dir,
    };
    Some(dir.join("config"))
}

fn parse_config(bytes: &[u8]) -> Result<gix::config::File, String> {
    gix::config::File::from_bytes_no_includes(
        bytes,
        gix::config::file::Metadata::from(gix::config::Source::Local),
        Default::default(),
    )
    .map_err(|e| e.to_string())
}

fn object_format(config: &gix::config::File) -> Option<String> {
    config
        .string("extensions.objectformat")
        .map(|v| v.to_string())
}

#[cfg(test)]
fn parse_object_format(config: &str) -> Option<String> {
    parse_config(config.as_bytes())
        .ok()
        .and_then(|c| object_format(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(hex: &str) -> ObjectId {
        ObjectId::from_hex(hex.as_bytes()).unwrap()
    }

    #[test]
    fn canonical_hash_matches_known_git_ids() {
        verify_hash(
            id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
            Kind::Blob,
            b"",
        )
        .unwrap();
        verify_hash(
            id("4b825dc642cb6eb9a060e54bf8d69288fbee4904"),
            Kind::Tree,
            b"",
        )
        .unwrap();
    }

    #[test]
    fn security_hash_mismatch_and_type_mismatch_are_refused() {
        let empty_blob = id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
        assert!(matches!(
            verify_hash(empty_blob, Kind::Blob, b"x"),
            Err(CannotRead::Substituted { .. })
        ));
        assert!(matches!(
            verify_hash(empty_blob, Kind::Tree, b""),
            Err(CannotRead::Substituted { .. })
        ));
        assert!(matches!(
            verify_kind(empty_blob, Kind::Tree, Kind::Blob),
            Err(CannotRead::TypeMismatch { .. })
        ));
    }

    #[test]
    fn commit_header_parsing() {
        let t = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
        let p = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        let data =
            format!("tree {t}\nparent {p}\nparent {t}\nauthor a <a> 0 +0000\n\nparent {p}\n");
        let c = parse_commit(id(p), data.as_bytes()).unwrap();
        assert_eq!(c.tree, id(t));
        assert_eq!(c.parents, vec![id(p), id(t)]);
        assert!(parse_commit(id(p), b"author x\n").is_err());
        assert!(parse_commit(id(p), format!("tree {}\n", &t[..39]).as_bytes()).is_err());
    }

    /// The names `git fsck --strict` 2.53.0 reports as `hasDotgit` (or refuses outright), and
    /// look-alikes it accepts (R-1b.4 oracle run, `work/names_oracle.py`).
    #[test]
    fn security_unsafe_entry_names_match_git_fsck() {
        let refused: &[&[u8]] = &[
            b"",
            b".",
            b"..",
            b"a/b",
            b".git",
            b".GIT",
            b".Git",
            b"git~1",
            b"GIT~1",
            b"git~1.",
            b".git.",
            b".git ",
            b".git. .",
            b".git::$INDEX_ALLOCATION",
            b".git:x",
            ".g\u{200c}it".as_bytes(),
            "\u{200c}.git".as_bytes(),
            ".git\u{feff}".as_bytes(),
            ".GI\u{200d}T".as_bytes(),
            b"a\\.git",
            b".git\\a",
            b"a\\git~1",
            b".git\\",
            b"\\.git",
        ];
        for name in refused {
            assert!(
                unsafe_entry_name(name).is_some(),
                "{:?}",
                String::from_utf8_lossy(name)
            );
        }
        let accepted: &[&[u8]] = &[
            b"git~2",
            b".gitx",
            b"git",
            b".git~1",
            b"x.git",
            b".gitmodules",
            b"git~1x",
            b"...",
            b"a\\b",
            b"\xff.git",
        ];
        for name in accepted {
            assert_eq!(
                unsafe_entry_name(name),
                None,
                "{:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    #[test]
    fn tree_paths_and_object_format_declaration() {
        assert_eq!(
            split_tree_path("metrics/register").unwrap(),
            vec!["metrics", "register"]
        );
        for bad in ["", "/a", "a/", "a//b", "./a", "a/../b"] {
            assert!(split_tree_path(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_object_format("[core]\n\tbare = false\n[extensions]\n\tobjectFormat = sha256\n")
                .as_deref(),
            Some("sha256")
        );
        assert_eq!(
            parse_object_format("[core]\n\tobjectformat = sha256\n"),
            None
        );
    }
}
