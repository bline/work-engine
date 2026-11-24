//! Private, single-writer attempt archive. Campaign owns spend and the marks;
//! this archive orders observation and preserves the crash-before-send gap.

use crate::{Result, digest, refuse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slice_campaign::offline_qualification::QualificationHistoryKey;
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const MAX_RECORDS: u64 = 128;
const MAX_RECORD_BYTES: u64 = 512 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Operation {
    Create,
    Start,
    Stop,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    position: u64,
    previous: Option<String>,
    kind: Kind,
    data: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(super) enum Kind {
    Selection,
    Spent,
    Intended(Operation),
    // Synced *after* checked Campaign mark, *before* any request byte can be
    // written. It may survive if no send took place. Never a replay ticket.
    Armed(Operation),
    // Observed completion of the local request write, not server acceptance.
    Written(Operation),
    Reply(Operation),
    CreateIdentity,
    CreateChecked,
    Inspect,
    Serving,
    AttachWire,
    Stdout,
    Running,
    Applied,
    HeldCapture,
    HeldCoverage,
    LiveSubtree,
    StopResolution,
    Settlement,
    ScopeChecked,
    Verdict,
}

fn private(file: &File, directory: bool) -> Result<()> {
    let m = file.metadata()?;
    // Safety: geteuid has no memory or other preconditions.
    if m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || (directory && !m.is_dir())
        || (!directory && (!m.is_file() || m.nlink() != 1))
    {
        return refuse("acquisition custody is not private to the controller");
    }
    Ok(())
}

fn dir(path: &Path) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)?;
    private(&f, true)?;
    Ok(f)
}

pub(super) fn check_root(root: &Path) -> Result<File> {
    if !root.is_absolute() || fs::canonicalize(root)? != root {
        return refuse("acquisition root must be canonical and protected");
    }
    dir(root)
}

pub(super) struct Custody {
    path: PathBuf,
    dir: File,
    position: u64,
    previous: Option<String>,
}

impl Custody {
    pub(super) fn begin(root: &Path, attempt: &str) -> Result<Self> {
        let parent = check_root(root)?;
        if !super::full_hex(attempt) {
            return refuse("invalid attempt ID");
        }
        let path = root.join(attempt);
        DirBuilder::new().mode(0o700).create(&path)?;
        let dir = dir(&path)?;
        parent.sync_all()?;
        Ok(Self {
            path,
            dir,
            position: 0,
            previous: None,
        })
    }

    pub(super) fn next(&self) -> u64 {
        self.position + 1
    }

    pub(super) fn record(&mut self, kind: Kind, data: Value) -> Result<u64> {
        let position = self.next();
        let record = Record {
            position,
            previous: self.previous.clone(),
            kind,
            data,
        };
        let bytes = serde_json::to_vec(&record)?;
        if position > MAX_RECORDS || bytes.len() as u64 > MAX_RECORD_BYTES {
            return refuse("acquisition custody bound exceeded");
        }
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join(format!("{position:03}.json")))?;
        private(&f, false)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        self.dir.sync_all()?;
        self.previous = Some(digest(&bytes));
        self.position = position;
        Ok(position)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ArchiveState {
    // A backwards-linked prefix cannot establish that a suffix is present.
    VerifiedPrefix,
    UnavailableOrDamaged,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CurrentDisposition {
    NoSpend,
    // This lookup does not read the owner's current mark/contest frontier.
    SpentCurrentDispositionUnknown,
}

/// Historical observations, never a qualification or a reconciliation decision.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct History {
    pub spent: bool,
    pub observed_create_id: Option<String>,
    pub checked_create: bool,
    pub possibly_submitted: Vec<Operation>,
    pub checked_scope: bool,
    pub archive: ArchiveState,
    pub disposition: CurrentDisposition,
}

impl History {
    pub(super) fn unavailable(spent: bool) -> Self {
        Self {
            spent,
            observed_create_id: None,
            checked_create: false,
            possibly_submitted: Vec::new(),
            checked_scope: false,
            archive: ArchiveState::UnavailableOrDamaged,
            disposition: if spent {
                CurrentDisposition::SpentCurrentDispositionUnknown
            } else {
                CurrentDisposition::NoSpend
            },
        }
    }
}

pub(super) fn history(root: &Path, key: &QualificationHistoryKey, spent: bool) -> History {
    let mut out = History::unavailable(spent);
    if read_prefix(root, key, &mut out).is_ok() {
        out.archive = ArchiveState::VerifiedPrefix;
    }
    out
}

fn read_prefix(root: &Path, key: &QualificationHistoryKey, out: &mut History) -> Result<()> {
    check_root(root)?;
    if !super::full_hex(&key.attempt) {
        return refuse("invalid recovery subject");
    }
    let path = root.join(&key.attempt);
    let _dir = dir(&path)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(&path)? {
        if names.len() as u64 == MAX_RECORDS {
            return refuse("acquisition history exceeds bound");
        }
        names.push(entry?.file_name());
    }
    names.sort_unstable();
    // Missing, torn or inconsistent history is an error, not a negative fact.
    if names.is_empty() {
        return refuse("missing acquisition history");
    }
    let mut previous = None;
    for (index, name) in names.into_iter().enumerate() {
        let position = index as u64 + 1;
        if name != format!("{position:03}.json").as_str() {
            return refuse("noncontiguous acquisition history");
        }
        let f = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path.join(name))?;
        private(&f, false)?;
        let mut bytes = Vec::new();
        f.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            return refuse("oversize acquisition history record");
        }
        let r: Record = serde_json::from_slice(&bytes)?;
        if r.position != position
            || r.previous != previous
            || (position == 1) != (r.kind == Kind::Selection)
            || (position == 1 && r.data["history_key"] != serde_json::to_value(key)?)
        {
            return refuse("acquisition history subject, order or digest differs");
        }
        previous = Some(digest(&bytes));
        match r.kind {
            Kind::Spent | Kind::Armed(_) if !out.spent => {
                return refuse("Campaign spend contradicts archive");
            }
            Kind::Armed(op) => out.possibly_submitted.push(op),
            Kind::Reply(Operation::Create) if r.data["response"]["status"] == 201 => {
                let response: super::docker::Response =
                    serde_json::from_value(r.data["response"].clone())?;
                let id = super::create_reply_id(&response.body).map_err(super::invalid)?;
                if !out.possibly_submitted.contains(&Operation::Create)
                    || out.observed_create_id.is_some()
                {
                    return refuse("unexpected Create response in archive");
                }
                out.observed_create_id = Some(id);
            }
            Kind::CreateIdentity => {
                if r.data.as_str() != out.observed_create_id.as_deref() {
                    return refuse("recorded Create identity differs from reply");
                }
            }
            Kind::CreateChecked => out.checked_create = true,
            Kind::ScopeChecked => out.checked_scope = true,
            _ => {}
        }
    }
    Ok(())
}
