//! Durable selection mechanics for a held legacy source.
//!
//! C0 has no production constructor for the cut session or source witness. The
//! only complete cut is exercised by private synthetic tests in this crate.
//! A later deployment owner must supply the real source/launch guard.

#![allow(dead_code)] // C0 mechanism is deliberately unreachable in production.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{SqliteLifecycleStore, StoreError};

const VERSION: u32 = 1;
const LOCK: &str = "held-selection.lock";
const SELECTION: &str = "held-selection.json";
const INTENT: &str = "held-cut-intent.json";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LegacySelection {
    pub version: u32,
    pub generation: u64,
    pub root: PathBuf,
    pub owner_id: String,
    pub owner_revision: String,
    pub executable_sha256: String,
    pub config_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HeldSelection {
    pub version: u32,
    pub generation: u64,
    pub phase: String,
    pub destination_root: PathBuf,
    pub destination_store_id: String,
    pub destination_custody_revision: u64,
    pub handoff_id: String,
    pub capture_sha256: String,
    pub capsule_sha256: String,
    pub source_fence_sha256: String,
    pub predecessor_sha256: String,
    pub build_sha256: String,
    pub config_sha256: String,
    pub guard_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Selection {
    Legacy(LegacySelection),
    Held(HeldSelection),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CutSpec {
    pub predecessor: LegacySelection,
    pub handoff_id: String,
    pub destination_root: PathBuf,
    pub capture_sha256: String,
    pub capsule_sha256: String,
    pub source_fence_sha256: String,
    pub build_sha256: String,
    pub config_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CutPhase {
    Reserved,
    Fenced,
    Selected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CutIntent {
    version: u32,
    phase: CutPhase,
    spec: CutSpec,
    guard_id: String,
}

pub(crate) struct HeldCutSession {
    directory: PathBuf,
    lock: File,
    lock_dev: u64,
    lock_ino: u64,
    guard_id: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec(value).map_err(|_| StoreError::Rejected)
}

fn check_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn checked_file(path: &Path) -> Result<File, StoreError> {
    let link = fs::symlink_metadata(path).map_err(|_| StoreError::Unavailable)?;
    if !link.file_type().is_file() || link.nlink() != 1 {
        return Err(StoreError::Rejected);
    }
    let file = File::open(path).map_err(|_| StoreError::Unavailable)?;
    let opened = file.metadata().map_err(|_| StoreError::Unavailable)?;
    if opened.dev() != link.dev() || opened.ino() != link.ino() {
        return Err(StoreError::ClaimConflict);
    }
    Ok(file)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, StoreError> {
    use std::io::Read;
    let file = checked_file(path)?;
    let len = file.metadata().map_err(|_| StoreError::Unavailable)?.len();
    if len == 0 || len > 64 * 1024 {
        return Err(StoreError::Rejected);
    }
    let mut bytes = Vec::with_capacity(len as usize);
    file.take(len + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Unavailable)?;
    if bytes.len() != len as usize {
        return Err(StoreError::ClaimConflict);
    }
    Ok(bytes)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, StoreError> {
    serde_json::from_slice(&read_bytes(path)?).map_err(|_| StoreError::Rejected)
}

fn sync_dir(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| StoreError::Unavailable)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WriteKind {
    Reserve,
    Fence,
    Selector,
    SelectedIntent,
    TestSetup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WriteCut {
    Create,
    Write,
    FileSync,
    StageDirSync,
    Rename,
    PublishDirSync,
}

// Fault injection is absent from production builds. Each test interruption
// occurs after the named real filesystem action; the next session must read
// and recover the resulting bytes under the same lock identity.
#[cfg(test)]
thread_local! {
    static TEST_CUT: std::cell::RefCell<Option<(WriteKind, WriteCut)>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn arm_test_cut(kind: WriteKind, cut: WriteCut) {
    TEST_CUT.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "previous selector test cut was not reached"
        );
        *slot.borrow_mut() = Some((kind, cut));
    });
}

#[cfg(test)]
pub(crate) fn clear_test_cut() {
    TEST_CUT.with(|slot| {
        assert!(slot.borrow().is_none(), "selector test cut was not reached");
    });
}

fn after_write_boundary(kind: WriteKind, cut: WriteCut) -> Result<(), StoreError> {
    #[cfg(test)]
    if TEST_CUT.with(|slot| *slot.borrow() == Some((kind, cut))) {
        TEST_CUT.with(|slot| *slot.borrow_mut() = None);
        return Err(StoreError::Unavailable);
    }
    let _ = (kind, cut);
    Ok(())
}

fn write_atomic_for(path: &Path, bytes: &[u8], kind: WriteKind) -> Result<(), StoreError> {
    let parent = path.parent().ok_or(StoreError::Rejected)?;
    if bytes.is_empty() || bytes.len() > 64 * 1024 {
        return Err(StoreError::Rejected);
    }
    let temporary = path.with_file_name(format!(
        "{}.next",
        path.file_name()
            .ok_or(StoreError::Rejected)?
            .to_string_lossy()
    ));
    if fs::symlink_metadata(&temporary).is_ok() {
        // A fully written stage belongs to exactly one intended revision and
        // can be replayed after a crash. A torn or conflicting stage is held.
        if read_bytes(&temporary)? != bytes {
            return Err(StoreError::ClaimConflict);
        }
        checked_file(&temporary)?
            .sync_all()
            .map_err(|_| StoreError::Unavailable)?;
        after_write_boundary(kind, WriteCut::FileSync)?;
    } else {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| StoreError::Unavailable)?;
        after_write_boundary(kind, WriteCut::Create)?;
        file.write_all(bytes).map_err(|_| StoreError::Unavailable)?;
        after_write_boundary(kind, WriteCut::Write)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        after_write_boundary(kind, WriteCut::FileSync)?;
    }
    sync_dir(parent)?;
    after_write_boundary(kind, WriteCut::StageDirSync)?;
    fs::rename(&temporary, path).map_err(|_| StoreError::Unavailable)?;
    after_write_boundary(kind, WriteCut::Rename)?;
    sync_dir(parent)?;
    after_write_boundary(kind, WriteCut::PublishDirSync)
}

#[cfg(test)]
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    write_atomic_for(path, bytes, WriteKind::TestSetup)
}

fn validate_legacy(record: &LegacySelection) -> Result<(), StoreError> {
    if record.version != VERSION
        || !record.root.is_absolute()
        || record.owner_id.is_empty()
        || record.owner_revision.is_empty()
        || !check_sha(&record.executable_sha256)
        || !check_sha(&record.config_sha256)
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

fn validate_spec(spec: &CutSpec) -> Result<(), StoreError> {
    validate_legacy(&spec.predecessor)?;
    if spec.handoff_id.is_empty()
        || !spec.destination_root.is_absolute()
        || spec.predecessor.root == spec.destination_root
        || !check_sha(&spec.capture_sha256)
        || !check_sha(&spec.capsule_sha256)
        || !check_sha(&spec.source_fence_sha256)
        || !check_sha(&spec.build_sha256)
        || !check_sha(&spec.config_sha256)
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

impl HeldCutSession {
    // C0 has no production opener. C1 must add a concrete verifier-acquired
    // opener, not a public trait implementation or caller-supplied token.
    #[cfg(test)]
    pub(crate) fn synthetic_initialize(
        directory: &Path,
        legacy: &LegacySelection,
    ) -> Result<Self, StoreError> {
        validate_legacy(legacy)?;
        if legacy.generation != 0 {
            return Err(StoreError::Rejected);
        }
        fs::create_dir(directory).map_err(|_| StoreError::ClaimConflict)?;
        sync_dir(directory.parent().ok_or(StoreError::Rejected)?)?;
        let session = Self::synthetic_resume(directory)?;
        let selection = session.directory.join(SELECTION);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(selection)
            .map_err(|_| StoreError::ClaimConflict)?;
        file.write_all(&encoded(&Selection::Legacy(legacy.clone()))?)
            .and_then(|_| file.sync_all())
            .map_err(|_| StoreError::Unavailable)?;
        sync_dir(&session.directory)?;
        Ok(session)
    }

    #[cfg(test)]
    pub(crate) fn synthetic_resume(directory: &Path) -> Result<Self, StoreError> {
        let canonical = fs::canonicalize(directory).map_err(|_| StoreError::Unavailable)?;
        if canonical != directory
            || !fs::symlink_metadata(directory)
                .map_err(|_| StoreError::Unavailable)?
                .file_type()
                .is_dir()
        {
            return Err(StoreError::Rejected);
        }
        let lock_path = canonical.join(LOCK);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|_| StoreError::Unavailable)?;
        lock.lock().map_err(|_| StoreError::Unavailable)?;
        let linked = fs::symlink_metadata(&lock_path).map_err(|_| StoreError::Unavailable)?;
        let opened = lock.metadata().map_err(|_| StoreError::Unavailable)?;
        if !linked.file_type().is_file()
            || linked.nlink() != 1
            || linked.dev() != opened.dev()
            || linked.ino() != opened.ino()
        {
            return Err(StoreError::ClaimConflict);
        }
        let guard_id =
            sha(format!("{}:{}:{}", canonical.display(), opened.dev(), opened.ino()).as_bytes());
        Ok(Self {
            directory: canonical,
            lock,
            lock_dev: opened.dev(),
            lock_ino: opened.ino(),
            guard_id,
        })
    }

    fn check_guard(&self) -> Result<(), StoreError> {
        let meta =
            fs::symlink_metadata(self.directory.join(LOCK)).map_err(|_| StoreError::Unavailable)?;
        let open = self.lock.metadata().map_err(|_| StoreError::Unavailable)?;
        if !meta.file_type().is_file()
            || meta.nlink() != 1
            || meta.dev() != self.lock_dev
            || meta.ino() != self.lock_ino
            || open.dev() != self.lock_dev
            || open.ino() != self.lock_ino
        {
            return Err(StoreError::ClaimConflict);
        }
        Ok(())
    }

    fn selection(&self) -> Result<Selection, StoreError> {
        self.check_guard()?;
        let selection: Selection = read_json(&self.directory.join(SELECTION))?;
        match &selection {
            Selection::Legacy(value) => validate_legacy(value)?,
            Selection::Held(value) => {
                if value.version != VERSION
                    || value.phase != "selected_recovery_only"
                    || !value.destination_root.is_absolute()
                    || value.destination_store_id.is_empty()
                    || value.handoff_id.is_empty()
                    || ![
                        &value.capture_sha256,
                        &value.capsule_sha256,
                        &value.source_fence_sha256,
                        &value.predecessor_sha256,
                        &value.build_sha256,
                        &value.config_sha256,
                        &value.guard_id,
                    ]
                    .iter()
                    .all(|s| check_sha(s))
                {
                    return Err(StoreError::Rejected);
                }
            }
        }
        Ok(selection)
    }

    fn intent(&self) -> Result<Option<CutIntent>, StoreError> {
        self.check_guard()?;
        let path = self.directory.join(INTENT);
        if !path.exists() {
            return Ok(None);
        }
        let intent: CutIntent = read_json(&path)?;
        if intent.version != VERSION || intent.guard_id != self.guard_id {
            return Err(StoreError::ClaimConflict);
        }
        validate_spec(&intent.spec)?;
        Ok(Some(intent))
    }

    fn held_matches_spec(
        &self,
        selected: &HeldSelection,
        spec: &CutSpec,
    ) -> Result<bool, StoreError> {
        Ok(selected.version == VERSION
            && selected.generation
                == spec
                    .predecessor
                    .generation
                    .checked_add(1)
                    .ok_or(StoreError::Rejected)?
            && selected.phase == "selected_recovery_only"
            && selected.destination_root == spec.destination_root
            && selected.handoff_id == spec.handoff_id
            && selected.capture_sha256 == spec.capture_sha256
            && selected.capsule_sha256 == spec.capsule_sha256
            && selected.source_fence_sha256 == spec.source_fence_sha256
            && selected.predecessor_sha256
                == sha(&encoded(&Selection::Legacy(spec.predecessor.clone()))?)
            && selected.build_sha256 == spec.build_sha256
            && selected.config_sha256 == spec.config_sha256
            && selected.guard_id == self.guard_id)
    }

    pub(crate) fn reserve(&self, spec: CutSpec) -> Result<(), StoreError> {
        validate_spec(&spec)?;
        let selected = self.selection()?;
        if let Some(prior) = self.intent()? {
            if prior.spec != spec {
                return Err(StoreError::ClaimConflict);
            }
            let matches = match (&prior.phase, &selected) {
                (CutPhase::Reserved | CutPhase::Fenced, Selection::Legacy(value)) => {
                    *value == spec.predecessor
                }
                (CutPhase::Fenced | CutPhase::Selected, Selection::Held(value)) => {
                    self.held_matches_spec(value, &spec)?
                }
                _ => false,
            };
            if !matches {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(());
        }
        if selected != Selection::Legacy(spec.predecessor.clone()) {
            return Err(StoreError::ClaimConflict);
        }
        let intent = CutIntent {
            version: VERSION,
            phase: CutPhase::Reserved,
            spec,
            guard_id: self.guard_id.clone(),
        };
        self.check_guard()?;
        write_atomic_for(
            &self.directory.join(INTENT),
            &encoded(&intent)?,
            WriteKind::Reserve,
        )?;
        self.check_guard()
    }

    /// C0's only fence issuer is private test code. C1 must replace this with
    /// concrete deployment/source-fence readback while holding this guard.
    #[cfg(test)]
    pub(crate) fn synthetic_fence(&self, fence_sha256: &str) -> Result<(), StoreError> {
        let mut intent = self.intent()?.ok_or(StoreError::Rejected)?;
        if fence_sha256 != intent.spec.source_fence_sha256 {
            return Err(StoreError::ClaimConflict);
        }
        let selection = self.selection()?;
        let matches = match (&intent.phase, &selection) {
            (CutPhase::Reserved | CutPhase::Fenced, Selection::Legacy(value)) => {
                *value == intent.spec.predecessor
            }
            (CutPhase::Fenced | CutPhase::Selected, Selection::Held(value)) => {
                self.held_matches_spec(value, &intent.spec)?
            }
            _ => false,
        };
        if !matches {
            return Err(StoreError::ClaimConflict);
        }
        if intent.phase == CutPhase::Reserved {
            intent.phase = CutPhase::Fenced;
            self.check_guard()?;
            write_atomic_for(
                &self.directory.join(INTENT),
                &encoded(&intent)?,
                WriteKind::Fence,
            )?;
            self.check_guard()?;
        }
        Ok(())
    }

    fn expected_held(
        &self,
        spec: &CutSpec,
        store: &SqliteLifecycleStore,
    ) -> Result<HeldSelection, StoreError> {
        let projection = store.held_projection()?;
        if projection.ownership_status != "inactive"
            || projection.handoff_id != spec.handoff_id
            || projection.source_capture_sha256 != spec.capture_sha256
            || projection.capsule_sha256 != spec.capsule_sha256
            || store.root() != spec.destination_root
            || projection.unclassified_count == 0
        {
            return Err(StoreError::ClaimConflict);
        }
        Ok(HeldSelection {
            version: VERSION,
            generation: spec
                .predecessor
                .generation
                .checked_add(1)
                .ok_or(StoreError::Rejected)?,
            phase: "selected_recovery_only".into(),
            destination_root: spec.destination_root.clone(),
            destination_store_id: projection.store_id,
            destination_custody_revision: projection.custody_revision,
            handoff_id: spec.handoff_id.clone(),
            capture_sha256: spec.capture_sha256.clone(),
            capsule_sha256: spec.capsule_sha256.clone(),
            source_fence_sha256: spec.source_fence_sha256.clone(),
            predecessor_sha256: sha(&encoded(&Selection::Legacy(spec.predecessor.clone()))?),
            build_sha256: spec.build_sha256.clone(),
            config_sha256: spec.config_sha256.clone(),
            guard_id: self.guard_id.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn synthetic_select(
        &self,
        store: &SqliteLifecycleStore,
        cut_after_publish: bool,
    ) -> Result<HeldSelection, StoreError> {
        let mut intent = self.intent()?.ok_or(StoreError::Rejected)?;
        if intent.phase == CutPhase::Reserved {
            return Err(StoreError::Rejected);
        }
        let expected = self.expected_held(&intent.spec, store)?;
        match self.selection()? {
            Selection::Legacy(prior)
                if intent.phase == CutPhase::Fenced && prior == intent.spec.predecessor =>
            {
                self.check_guard()?;
                write_atomic_for(
                    &self.directory.join(SELECTION),
                    &encoded(&Selection::Held(expected.clone()))?,
                    WriteKind::Selector,
                )?;
                self.check_guard()?;
                if cut_after_publish {
                    return Err(StoreError::Unavailable);
                }
            }
            Selection::Held(prior) if prior == expected => {}
            _ => return Err(StoreError::ClaimConflict),
        }
        if intent.phase != CutPhase::Selected {
            intent.phase = CutPhase::Selected;
            self.check_guard()?;
            write_atomic_for(
                &self.directory.join(INTENT),
                &encoded(&intent)?,
                WriteKind::SelectedIntent,
            )?;
            self.check_guard()?;
        }
        Ok(expected)
    }

    pub(crate) fn verify_selected(
        &self,
        store: &SqliteLifecycleStore,
    ) -> Result<HeldSelection, StoreError> {
        for name in [INTENT, SELECTION] {
            let pending = self.directory.join(format!("{name}.next"));
            if fs::symlink_metadata(pending).is_ok() {
                return Err(StoreError::ClaimConflict);
            }
        }
        let intent = self.intent()?.ok_or(StoreError::Rejected)?;
        if intent.phase != CutPhase::Selected {
            return Err(StoreError::ClaimConflict);
        }
        let expected = self.expected_held(&intent.spec, store)?;
        if self.selection()? != Selection::Held(expected.clone()) {
            return Err(StoreError::ClaimConflict);
        }
        Ok(expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy(root: &Path) -> LegacySelection {
        LegacySelection {
            version: VERSION,
            generation: 0,
            root: root.to_path_buf(),
            owner_id: "synthetic-legacy-owner".into(),
            owner_revision: "rev:one".into(),
            executable_sha256: sha(b"old executable"),
            config_sha256: sha(b"old config"),
        }
    }

    fn spec(base: &Path, predecessor: LegacySelection) -> CutSpec {
        CutSpec {
            predecessor,
            handoff_id: "handoff:synthetic-cut".into(),
            destination_root: base.join("absent-held-root"),
            capture_sha256: sha(b"synthetic capture"),
            capsule_sha256: sha(b"synthetic A-v2 capsule"),
            source_fence_sha256: sha(b"synthetic fence"),
            build_sha256: sha(b"recovery-only build"),
            config_sha256: sha(b"recovery-only config"),
        }
    }

    #[test]
    fn reservation_is_exact_and_source_revision_change_blocks_fence() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let predecessor = legacy(&source);
        let root = temp.path().join("selection");
        let session = HeldCutSession::synthetic_initialize(&root, &predecessor).unwrap();
        let cut = spec(temp.path(), predecessor.clone());
        session.reserve(cut.clone()).unwrap();
        session.reserve(cut.clone()).unwrap();
        assert!(session.synthetic_fence(&sha(b"wrong fence")).is_err());
        let mut conflicting = cut.clone();
        conflicting.handoff_id = "handoff:other".into();
        assert!(session.reserve(conflicting).is_err());
        let mut changed = predecessor;
        changed.owner_revision = "rev:two".into();
        write_atomic(
            &root.join(SELECTION),
            &encoded(&Selection::Legacy(changed)).unwrap(),
        )
        .unwrap();
        assert!(session.synthetic_fence(&cut.source_fence_sha256).is_err());
        assert!(session.reserve(cut).is_err());
    }

    #[test]
    fn replaced_lock_and_unknown_selection_format_refuse() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let root = temp.path().join("selection");
        let session = HeldCutSession::synthetic_initialize(&root, &legacy(&source)).unwrap();
        let bytes = fs::read(root.join(SELECTION)).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["unexpected"] = serde_json::json!(true);
        write_atomic(&root.join(SELECTION), &encoded(&value).unwrap()).unwrap();
        assert!(session.selection().is_err());
        fs::rename(root.join(LOCK), root.join("old-lock")).unwrap();
        File::create(root.join(LOCK)).unwrap();
        assert!(session.check_guard().is_err());
    }

    #[test]
    fn second_cooperating_selector_waits_for_same_stable_lock() {
        use std::sync::mpsc;
        use std::time::Duration;

        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let root = temp.path().join("selection");
        let first = HeldCutSession::synthetic_initialize(&root, &legacy(&source)).unwrap();
        let (sender, receiver) = mpsc::channel();
        let contender = std::thread::spawn(move || {
            let second = HeldCutSession::synthetic_resume(&root).unwrap();
            sender.send(second.guard_id).unwrap();
        });
        assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
        let first_id = first.guard_id.clone();
        drop(first);
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
            first_id
        );
        contender.join().unwrap();
    }

    #[test]
    fn fully_written_intent_stage_recovers_but_torn_stage_refuses() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let predecessor = legacy(&source);
        let root = temp.path().join("selection");
        let session = HeldCutSession::synthetic_initialize(&root, &predecessor).unwrap();
        let cut = spec(temp.path(), predecessor);
        let staged = CutIntent {
            version: VERSION,
            phase: CutPhase::Reserved,
            spec: cut.clone(),
            guard_id: session.guard_id.clone(),
        };
        fs::write(
            root.join(format!("{INTENT}.next")),
            encoded(&staged).unwrap(),
        )
        .unwrap();
        session.reserve(cut.clone()).unwrap();
        assert!(!root.join(format!("{INTENT}.next")).exists());
        assert_eq!(session.intent().unwrap().unwrap(), staged);
        let torn = root.join(format!("{INTENT}.next"));
        fs::write(&torn, b"{\"version\":").unwrap();
        assert!(session.synthetic_fence(&cut.source_fence_sha256).is_err());
        let mut conflict = cut;
        conflict.handoff_id = "handoff:other".into();
        assert!(
            write_atomic(
                &root.join(INTENT),
                &encoded(&CutIntent {
                    version: VERSION,
                    phase: CutPhase::Reserved,
                    spec: conflict,
                    guard_id: session.guard_id.clone(),
                })
                .unwrap()
            )
            .is_err()
        );
    }
}
