//! Same-generation AppArmor root-policy introspection for the stock natural
//! qualification. A task label alone cannot identify the loaded rules.

use super::{Result, full_hex, invalid, refuse, within};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Instant,
};

const POLICY: &str = "/sys/kernel/security/apparmor/policy";
const AAFS_MAGIC: libc::c_long = 0x5a3c69f0;

fn fd_path(file: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

fn open(path: &Path, directory: bool, follow: bool) -> Result<File> {
    let flags = libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 }
        | if follow { 0 } else { libc::O_NOFOLLOW };
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: file remains open and fs is valid writable syscall storage.
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } != 0 || fs.f_type != AAFS_MAGIC {
        return refuse("loaded AppArmor policy is not on the kernel policy filesystem");
    }
    Ok(file)
}

fn leaf(dir: &File, name: &str, limit: u64) -> Result<String> {
    let file = open(&fd_path(dir).join(name), false, false)?;
    let mut raw = String::new();
    file.take(limit + 1).read_to_string(&mut raw)?;
    if raw.len() as u64 > limit || !raw.ends_with('\n') {
        return refuse("invalid loaded AppArmor profile field");
    }
    Ok(raw)
}

fn revision(root: &File) -> Result<u64> {
    // AAFS revision reports a change, not ordinary EOF on a second read.
    // Open fresh and read once with O_NONBLOCK so a missing generation fails.
    let mut file = open(&fd_path(root).join("revision"), false, false)?;
    let mut bytes = [0; 32];
    let n = file.read(&mut bytes)?;
    let text = std::str::from_utf8(&bytes[..n]).map_err(invalid)?;
    parse_revision(text).ok_or_else(|| invalid("invalid AppArmor namespace revision"))
}

fn parse_revision(text: &str) -> Option<u64> {
    let text = text.strip_suffix('\n')?;
    let rev: u64 = text.parse().ok()?;
    (rev != 0 && text == rev.to_string()).then_some(rev)
}

fn checked_hash(raw: &str) -> Option<&str> {
    raw.strip_suffix('\n').filter(|hash| full_hex(hash))
}

#[derive(Serialize)]
pub(super) struct LoadedProfile {
    directory: String,
    inode: u64,
    device: u64,
    revision: u64,
    name: String,
    mode: String,
    sha256: String,
}

impl LoadedProfile {
    pub(super) fn identify_task(&self, label: &[u8]) -> Result<(String, String)> {
        let name = self.name.strip_suffix('\n').ok_or_else(|| invalid("invalid loaded AppArmor name"))?;
        let mode = self.mode.strip_suffix('\n').ok_or_else(|| invalid("invalid loaded AppArmor mode"))?;
        if mode != "enforce" || label != format!("{name} ({mode})\n").as_bytes() {
            return refuse("original task label does not match the enforcing loaded policy");
        }
        Ok((name.into(), self.sha256.clone()))
    }
}

pub(super) struct AppArmor {
    root: File,
    profile: File,
    generation: LoadedProfile,
}

impl AppArmor {
    pub(super) fn pin(name: &str, sha256: &str, deadline: Instant) -> Result<Self> {
        within(deadline)?;
        if name != "docker-default" || !full_hex(sha256) {
            return refuse("stock natural exit requires a pinned docker-default AppArmor profile");
        }
        // AAFS exposes `policy` via a kernel dynamic link, then all profile
        // files are read through the retained namespace and directory FDs.
        let root = open(Path::new(POLICY), true, true)?;
        let before = revision(&root)?;
        let profiles = open(&fd_path(&root).join("profiles"), true, false)?;
        let mut found = None;
        for (index, entry) in fs::read_dir(fd_path(&profiles))?.enumerate() {
            within(deadline)?;
            if index >= 512 {
                return refuse(
                    "AppArmor namespace profile inventory exceeds the qualification bound",
                );
            }
            let directory = entry?.file_name();
            let Some(text) = directory.to_str() else {
                continue;
            };
            if !text.strip_prefix("docker-default.").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
            }) {
                continue;
            }
            let profile = open(&fd_path(&profiles).join(&directory), true, false)?;
            let label = leaf(&profile, "name", 128)?;
            if label != "docker-default\n" {
                continue;
            }
            if found.replace((directory, profile, label)).is_some() {
                return refuse("ambiguous active docker-default AppArmor profile");
            }
        }
        let (directory, profile, label) =
            found.ok_or_else(|| invalid("active docker-default profile unavailable"))?;
        let mode = leaf(&profile, "mode", 32)?;
        let hash_line = leaf(&profile, "sha256", 65)?;
        let loaded_sha256 = checked_hash(&hash_line)
            .ok_or_else(|| invalid("loaded AppArmor profile lacks a complete kernel hash"))?;
        if mode != "enforce\n" || loaded_sha256 != sha256 || revision(&root)? != before {
            return refuse(
                "loaded AppArmor mode/hash differs from the pin, or policy changed during acquisition",
            );
        }
        let meta = profile.metadata()?;
        within(deadline)?;
        Ok(Self {
            root,
            profile,
            generation: LoadedProfile {
                directory: directory
                    .into_string()
                    .map_err(|_| invalid("invalid AppArmor profile entry"))?,
                inode: meta.ino(),
                device: meta.dev(),
                revision: before,
                name: label,
                mode,
                sha256: loaded_sha256.into(),
            },
        })
    }

    pub(super) fn check(&self, deadline: Instant) -> Result<&LoadedProfile> {
        within(deadline)?;
        let current = open(Path::new(POLICY), true, true)?.metadata()?;
        let original = self.root.metadata()?;
        if (current.dev(), current.ino()) != (original.dev(), original.ino())
            || revision(&self.root)? != self.generation.revision
            || leaf(&self.profile, "name", 128)? != self.generation.name
            || leaf(&self.profile, "mode", 32)? != self.generation.mode
            || checked_hash(&leaf(&self.profile, "sha256", 65)?)
                != Some(self.generation.sha256.as_str())
            || revision(&self.root)? != self.generation.revision
        {
            return refuse("original AppArmor policy generation did not persist");
        }
        within(deadline)?;
        Ok(&self.generation)
    }

    pub(super) fn generation(&self) -> &LoadedProfile {
        &self.generation
    }
}

#[cfg(test)]
#[test]
fn stock_apparmor_fields_require_complete_kernel_revision_and_hash() {
    assert_eq!(parse_revision("310\n"), Some(310));
    for invalid in ["", "0\n", "310", "0310\n", "+310\n", "310\n311\n"] {
        assert_eq!(parse_revision(invalid), None);
    }
    let approved = "a".repeat(64);
    assert_eq!(
        checked_hash(&format!("{approved}\n")),
        Some(approved.as_str())
    );
    assert!(checked_hash(&approved).is_none());
    assert!(checked_hash(&format!("{}\n", "A".repeat(64))).is_none());
}
