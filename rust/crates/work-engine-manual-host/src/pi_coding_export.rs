//! In-band bounded export collection for the signed fixture profile. The container's `/export`
//! is a size-limited tmpfs that the Host cannot read, so the owned shim sends its files over the
//! existing stdio protocol before it reports completion. Every frame is checked against a
//! persisted state machine and the bytes are stored by the Host itself. Exports are provisional
//! evidence authored by the sandbox, never attestation.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

/// Size of the container's `/export` tmpfs and the aggregate bound the Host accepts.
pub const MAX_TOTAL: u64 = 1_048_576;
pub const MAX_FILES: usize = 16;
pub const MAX_PATH: usize = 128;
pub const MAX_COMPONENT: usize = 80;
pub const MAX_DEPTH: usize = 6;
/// Raw bytes per part. Base64 of this plus metadata stays far below the 64 KiB frame bound.
pub const MAX_PART: usize = 24_576;
pub const MAX_PARTS: u64 = 128;
/// The exact tmpfs option string the Host creates and then verifies by inspection.
/// The `classification` every frame of the signed fixture shim carries (shim `emit`).
pub const CLASSIFICATION: &str = "fixture provider/signing/identity; actual SDK runtime";
pub const TMPFS: &str = "/export:rw,noexec,nosuid,nodev,size=1048576,uid=1000,gid=1000,mode=0700";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFile {
    pub total: u64,
    pub sha256: String,
    pub received: u64,
    pub done: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportState {
    pub next_seq: u64,
    pub files: BTreeMap<String, ExportFile>,
    /// Path currently receiving parts. A new path may not begin until it is done.
    pub open: Option<String>,
    pub total_bytes: u64,
    pub finalized: bool,
    pub manifest_sha256: Option<String>,
}

/// Canonical manifest digest over (path, size, sha256) of every file, ordered by path.
pub fn manifest_digest(files: &BTreeMap<String, ExportFile>) -> String {
    let mut text = String::new();
    for (path, f) in files {
        text.push_str(&format!("{path}\t{}\t{}\n", f.total, f.sha256));
    }
    digest(text.as_bytes())
}

/// Relative, normalized, bounded path. No traversal, no absolute path, no empty or dot component,
/// a closed character set, and not the Host's own `workspace/` export namespace.
pub fn valid_path(path: &str) -> bool {
    if path.is_empty() || path.len() > MAX_PATH {
        return false;
    }
    let parts: Vec<&str> = path.split('/').collect();
    parts.len() <= MAX_DEPTH
        && parts[0] != "workspace"
        && parts.iter().all(|c| {
            !c.is_empty()
                && c.len() <= MAX_COMPONENT
                && *c != "."
                && *c != ".."
                && c.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}

fn exact_keys(frame: &Value, keys: &[&str]) -> bool {
    let Some(map) = frame.as_object() else {
        return false;
    };
    let mut got: Vec<&str> = map.keys().map(String::as_str).collect();
    // Every shim frame carries the fixed classification; it is part of the exact shape.
    let mut want = keys.to_vec();
    want.push("classification");
    if frame["classification"] != CLASSIFICATION {
        return false;
    }
    got.sort_unstable();
    want.sort_unstable();
    got == want
}
fn is_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Host {
    pub(in crate::pi_docker) fn is_signed_fixture(&self) -> bool {
        self.journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| p.schema == SIGNED_PROFILE)
    }
    pub(in crate::pi_docker) fn export_finalized(&self) -> bool {
        self.journal
            .coding
            .as_ref()
            .and_then(|c| c.export.as_ref())
            .is_some_and(|e| e.finalized)
    }
    /// Every collected export file (outside the Host's own `workspace/` namespace) is exactly one
    /// finalized in-band file with the same digest, and none is missing.
    pub(in crate::pi_docker) fn export_collection_consistent(&self) -> bool {
        let Some(e) = self.journal.coding.as_ref().and_then(|c| c.export.as_ref()) else {
            return false;
        };
        let collected: BTreeMap<&str, &str> = self
            .journal
            .exported_files
            .iter()
            .filter(|(k, _)| !k.starts_with("workspace/"))
            .map(|(k, v)| (k.as_str(), v.sha256.as_str()))
            .collect();
        e.finalized
            && collected.len() == e.files.len()
            && e.files
                .iter()
                .all(|(p, f)| f.done && collected.get(p.as_str()) == Some(&f.sha256.as_str()))
    }
    /// Validates and applies one `export_part` or `export_final` frame.
    pub(super) fn coding_export_frame(&mut self, frame: &Value, kind: &str) -> Result<()> {
        if !self.is_signed_fixture() {
            return refuse("export frames exist only in the signed fixture profile");
        }
        if !self.journal.coding.as_ref().unwrap().ready_observed {
            return refuse("export frame before native readiness");
        }
        let mut state = self
            .journal
            .coding
            .as_ref()
            .unwrap()
            .export
            .clone()
            .unwrap_or_default();
        if state.finalized {
            return refuse("export frame after finalization");
        }
        if kind == "export_final" {
            if !exact_keys(
                frame,
                &[
                    "schema",
                    "sessionId",
                    "type",
                    "files",
                    "bytes",
                    "manifest_sha256",
                ],
            ) {
                return refuse("export final frame shape");
            }
            if state.open.is_some()
                || state.files.is_empty()
                || frame["files"].as_u64() != Some(state.files.len() as u64)
                || frame["bytes"].as_u64() != Some(state.total_bytes)
                || state
                    .files
                    .values()
                    .any(|f| !f.done || f.received != f.total)
                || frame["manifest_sha256"].as_str() != Some(&manifest_digest(&state.files))
            {
                return refuse("export finalization conflicts with received files");
            }
            state.finalized = true;
            state.manifest_sha256 = Some(manifest_digest(&state.files));
            self.journal.coding.as_mut().unwrap().export = Some(state);
            return Ok(());
        }
        if !exact_keys(
            frame,
            &[
                "schema",
                "sessionId",
                "type",
                "seq",
                "path",
                "offset",
                "total",
                "sha256",
                "data",
            ],
        ) {
            return refuse("export part frame shape");
        }
        let (Some(seq), Some(path), Some(offset), Some(total), Some(sha), Some(data)) = (
            frame["seq"].as_u64(),
            frame["path"].as_str(),
            frame["offset"].as_u64(),
            frame["total"].as_u64(),
            frame["sha256"].as_str(),
            frame["data"].as_str(),
        ) else {
            return refuse("export part field types");
        };
        if seq != state.next_seq || seq >= MAX_PARTS {
            return refuse("export part duplicated, reordered or beyond the part bound");
        }
        if !valid_path(path) {
            return refuse("export path is not a bounded relative path");
        }
        if !is_hash(sha) || total > MAX_TOTAL {
            return refuse("export part digest or size invalid");
        }
        let bytes = STANDARD
            .decode(data)
            .map_err(|_| Error::Refused("export part is not base64".into()))?;
        if STANDARD.encode(&bytes) != data
            || bytes.len() > MAX_PART
            || (bytes.is_empty() && total != 0)
        {
            return refuse("export part bytes non-canonical or oversized");
        }
        let root = self.journal.job.state_dir.join("exports");
        let target = root.join(path);
        if let Some(file) = state.files.get_mut(path) {
            if state.open.as_deref() != Some(path)
                || file.done
                || file.total != total
                || file.sha256 != sha
                || file.received != offset
            {
                return refuse("export part conflicts with its file or arrives out of order");
            }
            let mut out = OpenOptions::new().append(true).open(&target)?;
            if out.metadata()?.len() != offset {
                return refuse("export file length conflicts with the received offset");
            }
            out.write_all(&bytes)?;
            out.sync_all()?;
            file.received += bytes.len() as u64;
        } else {
            if state.open.is_some()
                || offset != 0
                || state.files.len() >= MAX_FILES
                || state
                    .total_bytes
                    .checked_add(total)
                    .is_none_or(|t| t > MAX_TOTAL)
            {
                return refuse("export file exceeds the file count or aggregate size bound");
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode_0600()
                .open(&target)?;
            out.write_all(&bytes)?;
            out.sync_all()?;
            state.total_bytes += total;
            state.files.insert(
                path.into(),
                ExportFile {
                    total,
                    sha256: sha.into(),
                    received: bytes.len() as u64,
                    done: false,
                },
            );
            state.open = Some(path.into());
        }
        let file = state.files.get_mut(path).expect("inserted");
        if file.received > file.total {
            return refuse("export file exceeds its declared size");
        }
        if file.received == file.total {
            // The digest is checked against the bytes the Host itself stored, not the message.
            let meta = fs::symlink_metadata(&target)?;
            if !meta.is_file() || meta.len() != file.total {
                return refuse("stored export is not the declared regular file");
            }
            if file_digest(&target, None)? != file.sha256 {
                return refuse("stored export digest conflicts with the declared digest");
            }
            file.done = true;
            state.open = None;
        }
        state.next_seq += 1;
        self.journal.coding.as_mut().unwrap().export = Some(state);
        Ok(())
    }
}

trait Mode0600 {
    fn mode_0600(&mut self) -> &mut Self;
}
impl Mode0600 for OpenOptions {
    fn mode_0600(&mut self) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;
        self.mode(0o600)
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    const SESSION: &str = "00000000-0000-4000-8000-000000000001";

    fn part(seq: u64, path: &str, offset: u64, total: u64, sha: &str, data: &[u8]) -> Value {
        json!({"schema":SIGNED_PROFILE,"sessionId":SESSION,"classification":CLASSIFICATION,"type":"export_part","seq":seq,"path":path,
               "offset":offset,"total":total,"sha256":sha,"data":STANDARD.encode(data)})
    }
    fn fin(files: u64, bytes: u64, manifest: &str) -> Value {
        json!({"schema":SIGNED_PROFILE,"sessionId":SESSION,"classification":CLASSIFICATION,"type":"export_final","files":files,
               "bytes":bytes,"manifest_sha256":manifest})
    }
    fn frames(values: &[Value]) -> Vec<u8> {
        let mut raw = vec![];
        for v in values {
            raw.extend(serde_json::to_vec(v).unwrap());
            raw.push(b'\n');
        }
        raw
    }
    fn host() -> (tempfile::TempDir, Host) {
        let (dir, mut h) = fixture();
        h.journal.job.coding.as_mut().unwrap().schema = SIGNED_PROFILE.into();
        h.journal.session_id = Some(SESSION.into());
        h.journal.coding.as_mut().unwrap().ready_observed = true;
        fs::create_dir(dir.path().join("exports")).unwrap();
        (dir, h)
    }
    fn advance(h: &mut Host, values: &[Value]) -> Result<()> {
        h.coding_advance(&frames(values)).map(|_| ())
    }
    fn sha(b: &[u8]) -> String {
        digest(b)
    }
    fn good_manifest(entries: &[(&str, &[u8])]) -> (BTreeMap<String, ExportFile>, u64) {
        let mut files = BTreeMap::new();
        let mut total = 0;
        for (p, b) in entries {
            files.insert(
                (*p).into(),
                ExportFile {
                    total: b.len() as u64,
                    sha256: sha(b),
                    received: b.len() as u64,
                    done: true,
                },
            );
            total += b.len() as u64;
        }
        (files, total)
    }
    fn two_files() -> Vec<Value> {
        vec![
            part(0, "a.json", 0, 3, &sha(b"abc"), b"abc"),
            part(1, "sessions/b.jsonl", 0, 4, &sha(b"wxyz"), b"wxyz"),
        ]
    }
    fn finalize_two() -> Value {
        let (files, total) = good_manifest(&[("a.json", b"abc"), ("sessions/b.jsonl", b"wxyz")]);
        fin(2, total, &manifest_digest(&files))
    }

    #[test]
    fn valid_stream_stores_exact_bytes_and_finalizes() {
        let (dir, mut h) = host();
        let mut all = two_files();
        all.push(finalize_two());
        advance(&mut h, &all).unwrap();
        assert!(h.export_finalized());
        assert_eq!(fs::read(dir.path().join("exports/a.json")).unwrap(), b"abc");
        assert_eq!(
            fs::read(dir.path().join("exports/sessions/b.jsonl")).unwrap(),
            b"wxyz"
        );
    }
    #[test]
    fn multi_part_file_is_reassembled_and_checked() {
        let (dir, mut h) = host();
        let whole = b"0123456789";
        let s = sha(whole);
        let (files, total) = good_manifest(&[("big.bin", whole)]);
        advance(
            &mut h,
            &[
                part(0, "big.bin", 0, 10, &s, &whole[..4]),
                part(1, "big.bin", 4, 10, &s, &whole[4..]),
                fin(1, total, &manifest_digest(&files)),
            ],
        )
        .unwrap();
        assert_eq!(fs::read(dir.path().join("exports/big.bin")).unwrap(), whole);
    }
    #[test]
    fn rejects_unsafe_and_unbounded_paths() {
        for bad in [
            "../x",
            "/abs",
            "a/../b",
            "a//b",
            "",
            ".",
            "a/./b",
            "workspace/x",
            "a b",
            "a\\b",
            "sessions/",
            "a/b/c/d/e/f/g",
        ] {
            let (_dir, mut h) = host();
            assert!(
                advance(&mut h, &[part(0, bad, 0, 1, &sha(b"a"), b"a")]).is_err(),
                "{bad}"
            );
        }
        let long = "a".repeat(MAX_PATH + 1);
        let (_dir, mut h) = host();
        assert!(advance(&mut h, &[part(0, &long, 0, 1, &sha(b"a"), b"a")]).is_err());
    }
    #[test]
    fn rejects_a_digest_that_does_not_match_the_stored_bytes() {
        let (_dir, mut h) = host();
        assert!(advance(&mut h, &[part(0, "a", 0, 3, &sha(b"abd"), b"abc")]).is_err());
    }
    #[test]
    fn rejects_aggregate_overflow_across_files_and_oversized_files() {
        // One byte below the aggregate bound is already used; another file of 2 bytes overflows it.
        let (_dir, mut h) = host();
        let mut st = ExportState::default();
        st.total_bytes = MAX_TOTAL - 1;
        st.next_seq = 1;
        st.files.insert(
            "x".into(),
            ExportFile {
                total: MAX_TOTAL - 1,
                sha256: sha(b"x"),
                received: MAX_TOTAL - 1,
                done: true,
            },
        );
        h.journal.coding.as_mut().unwrap().export = Some(st);
        assert!(advance_from(&mut h, 1, vec![part(1, "y", 0, 2, &sha(b"yy"), b"yy")]).is_err());
        // Exactly filling the bound is accepted.
        let (_dir, mut h) = host();
        let mut st = ExportState::default();
        st.total_bytes = MAX_TOTAL - 1;
        st.next_seq = 1;
        st.files.insert(
            "x".into(),
            ExportFile {
                total: MAX_TOTAL - 1,
                sha256: sha(b"x"),
                received: MAX_TOTAL - 1,
                done: true,
            },
        );
        h.journal.coding.as_mut().unwrap().export = Some(st);
        advance_from(&mut h, 1, vec![part(1, "y", 0, 1, &sha(b"y"), b"y")]).unwrap();
        // A single declared file over the aggregate bound.
        let (_dir, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[part(0, "huge", 0, MAX_TOTAL + 1, &sha(b"a"), b"a")]
            )
            .is_err()
        );
        // A part larger than the per-part bound.
        let (_dir, mut h) = host();
        let big = vec![1u8; MAX_PART + 1];
        assert!(
            advance(
                &mut h,
                &[part(0, "big", 0, big.len() as u64, &sha(&big), &big)]
            )
            .is_err()
        );
        // More parts than the part bound.
        let (_dir, mut h) = host();
        let mut st = ExportState::default();
        st.next_seq = MAX_PARTS;
        h.journal.coding.as_mut().unwrap().export = Some(st);
        assert!(
            advance_from(
                &mut h,
                MAX_PARTS as usize,
                vec![part(MAX_PARTS, "z", 0, 1, &sha(b"z"), b"z")]
            )
            .is_err()
        );
    }
    /// The frame counter resumes after frames already applied (those lines are skipped).
    fn advance_from(h: &mut Host, skip: usize, values: Vec<Value>) -> Result<()> {
        h.journal.coding.as_mut().unwrap().frames = skip;
        let mut raw = vec![];
        for _ in 0..skip {
            raw.extend(b"{}\n");
        }
        raw.extend(frames(&values));
        h.coding_advance(&raw).map(|_| ())
    }
    #[test]
    fn rejects_too_many_files() {
        let (_dir, mut h) = host();
        let mut values = vec![];
        for i in 0..=MAX_FILES as u64 {
            values.push(part(i, &format!("f{i}"), 0, 1, &sha(b"a"), b"a"));
        }
        assert!(advance(&mut h, &values).is_err());
        assert_eq!(
            h.journal
                .coding
                .as_ref()
                .unwrap()
                .export
                .as_ref()
                .unwrap()
                .files
                .len(),
            MAX_FILES
        );
    }
    #[test]
    fn rejects_duplicate_reordered_and_conflicting_parts() {
        // Duplicate seq.
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[two_files()[0].clone(), two_files()[0].clone()]).is_err());
        // Out-of-order seq (a gap).
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[part(1, "a", 0, 1, &sha(b"a"), b"a")]).is_err());
        // Same path as a completed file with a new seq: conflicting create.
        let (_d, mut h) = host();
        let a = part(0, "a", 0, 1, &sha(b"a"), b"a");
        let again = part(1, "a", 0, 1, &sha(b"a"), b"a");
        assert!(advance(&mut h, &[a, again]).is_err());
        // Continuation with a changed declared digest or total.
        let (_d, mut h) = host();
        let s = sha(b"abcd");
        assert!(
            advance(
                &mut h,
                &[
                    part(0, "a", 0, 4, &s, b"ab"),
                    part(1, "a", 2, 4, &sha(b"other"), b"cd"),
                ]
            )
            .is_err()
        );
        let (_d, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[part(0, "a", 0, 4, &s, b"ab"), part(1, "a", 2, 5, &s, b"cd")]
            )
            .is_err()
        );
        // Offset gap, and overlap.
        let (_d, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[part(0, "a", 0, 4, &s, b"ab"), part(1, "a", 3, 4, &s, b"d")]
            )
            .is_err()
        );
        let (_d, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[
                    part(0, "a", 0, 4, &s, b"ab"),
                    part(1, "a", 1, 4, &s, b"bcd")
                ]
            )
            .is_err()
        );
        // A new file while the previous one is still open (interleaving).
        let (_d, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[
                    part(0, "a", 0, 4, &s, b"ab"),
                    part(1, "b", 0, 1, &sha(b"b"), b"b")
                ]
            )
            .is_err()
        );
        // Bytes beyond the declared size.
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[part(0, "a", 0, 2, &sha(b"ab"), b"abc")]).is_err());
        // An empty part for a non-empty file.
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[part(0, "a", 0, 2, &sha(b"ab"), b"")]).is_err());
    }
    #[test]
    fn rejects_a_symlink_or_directory_collision_in_the_exports_root() {
        let (dir, mut h) = host();
        std::os::unix::fs::symlink("/etc/hostname", dir.path().join("exports/a")).unwrap();
        assert!(advance(&mut h, &[part(0, "a", 0, 1, &sha(b"a"), b"a")]).is_err());
        let (_dir, mut h) = host();
        assert!(
            advance(
                &mut h,
                &[
                    part(0, "a", 0, 1, &sha(b"a"), b"a"),
                    part(1, "a/b", 0, 1, &sha(b"b"), b"b")
                ]
            )
            .is_err()
        );
    }
    #[test]
    fn rejects_malformed_frames_and_bad_base64() {
        let mut extra = two_files()[0].clone();
        extra["extra"] = json!(1);
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[extra]).is_err());
        let mut noncanon = two_files()[0].clone();
        noncanon["data"] = json!("YWJj\n");
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[noncanon]).is_err());
        let mut bad = two_files()[0].clone();
        bad["data"] = json!("@@@");
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[bad]).is_err());
        let mut other_class = two_files()[0].clone();
        other_class["classification"] = json!("something else");
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[other_class]).is_err());
        let mut no_class = two_files()[0].clone();
        no_class.as_object_mut().unwrap().remove("classification");
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[no_class]).is_err());
        let mut upper = two_files()[0].clone();
        upper["sha256"] = json!(sha(b"abc").to_uppercase());
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[upper]).is_err());
    }
    #[test]
    fn rejects_bad_finalization() {
        let good = finalize_two();
        for (name, mutate) in [
            ("files", json!({"files":3})),
            ("bytes", json!({"bytes":8})),
            ("manifest", json!({"manifest_sha256":"0".repeat(64)})),
        ] {
            let mut f = good.clone();
            for (k, v) in mutate.as_object().unwrap() {
                f[k] = v.clone();
            }
            let (_d, mut h) = host();
            let mut all = two_files();
            all.push(f);
            assert!(advance(&mut h, &all).is_err(), "{name}");
            assert!(!h.export_finalized());
        }
        // Finalization before any file, while a file is open, and twice.
        let (_d, mut h) = host();
        assert!(advance(&mut h, &[fin(0, 0, &manifest_digest(&BTreeMap::new()))]).is_err());
        let (_d, mut h) = host();
        let s = sha(b"abcd");
        let mut st = vec![part(0, "a", 0, 4, &s, b"ab")];
        st.push(fin(1, 4, "0".repeat(64).as_str()));
        assert!(advance(&mut h, &st).is_err());
        let (_d, mut h) = host();
        let mut all = two_files();
        all.push(finalize_two());
        all.push(finalize_two());
        assert!(advance(&mut h, &all).is_err());
        // A part after finalization.
        let (_d, mut h) = host();
        let mut all = two_files();
        all.push(finalize_two());
        all.push(part(2, "c", 0, 1, &sha(b"c"), b"c"));
        assert!(advance(&mut h, &all).is_err());
    }
    /// A signed host whose native evidence is complete except for the export frames.
    fn complete_ready_host() -> (tempfile::TempDir, Host) {
        let (d, mut h) = host();
        let st = h.journal.coding.as_mut().unwrap();
        st.tool_requests.insert(
            "t".into(),
            Some(Artifact {
                path: "/x".into(),
                sha256: sha(b""),
            }),
        );
        st.native_tool_ends = 1;
        st.provider_requests.insert(
            1,
            provider::Reservation {
                reserved_microusd: 0,
                settled_microusd: Some(0),
                ..Default::default()
            },
        );
        (d, h)
    }
    fn complete_frame() -> Value {
        json!({"schema":SIGNED_PROFILE,"sessionId":SESSION,"type":"complete","provider":"fake_fixture"})
    }
    #[test]
    fn completion_requires_finalization_and_a_stream_ending_early_is_not_complete() {
        // Positive control: the same native evidence plus a finalized export completes.
        let (_d, mut h) = complete_ready_host();
        let mut all = two_files();
        all.push(finalize_two());
        all.push(complete_frame());
        advance(&mut h, &all).unwrap();
        assert!(h.journal.coding.as_ref().unwrap().completed);
        // Without export frames, `complete` alone is refused for the export reason only.
        let (_d, mut h) = complete_ready_host();
        let err = advance(&mut h, &[complete_frame()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("export finalization"), "{err}");
        assert!(!h.journal.coding.as_ref().unwrap().completed);
        // Files received but never finalized: `complete` is refused.
        let (_d, mut h) = complete_ready_host();
        let mut all = two_files();
        all.push(complete_frame());
        let err = advance(&mut h, &all).unwrap_err().to_string();
        assert!(err.contains("export finalization"), "{err}");
        // A stream that stops mid-export: no completion, no finalization, consistency is false.
        let (_d, mut h) = host();
        advance(&mut h, &two_files()).unwrap();
        assert!(!h.export_finalized());
        assert!(!h.export_collection_consistent());
        assert!(!h.journal.coding.as_ref().unwrap().completed);
        // A partial trailing line is not a frame: it is neither applied nor counted.
        let (_d, mut h) = host();
        let mut raw = frames(&two_files()[..1]);
        raw.extend(b"{\"schema\":\"partial");
        h.coding_advance(&raw).unwrap();
        assert_eq!(
            h.journal
                .coding
                .as_ref()
                .unwrap()
                .export
                .as_ref()
                .unwrap()
                .next_seq,
            1
        );
    }
    #[test]
    fn receipt_status_requires_finalized_and_consistent_collection() {
        let (_d, mut h) = host();
        assert!(!h.export_collection_consistent());
        let mut all = two_files();
        all.push(finalize_two());
        advance(&mut h, &all).unwrap();
        // Finalized, but the Host has not collected the files: not consistent.
        assert!(h.export_finalized() && !h.export_collection_consistent());
        let root = h.journal.job.state_dir.join("exports");
        for (p, b) in [("a.json", &b"abc"[..]), ("sessions/b.jsonl", &b"wxyz"[..])] {
            h.journal.exported_files.insert(
                p.into(),
                Artifact {
                    path: root.join(p),
                    sha256: sha(b),
                },
            );
        }
        assert!(h.export_collection_consistent());
        // A collected digest that differs from the finalized one is inconsistent.
        h.journal.exported_files.get_mut("a.json").unwrap().sha256 = sha(b"other");
        assert!(!h.export_collection_consistent());
        h.journal.exported_files.get_mut("a.json").unwrap().sha256 = sha(b"abc");
        // An extra collected file outside the finalized set is inconsistent; workspace/ is exempt.
        h.journal.exported_files.insert(
            "workspace/src/lib.rs".into(),
            Artifact {
                path: root.join("w"),
                sha256: sha(b"w"),
            },
        );
        assert!(h.export_collection_consistent());
        h.journal.exported_files.insert(
            "extra".into(),
            Artifact {
                path: root.join("extra"),
                sha256: sha(b"e"),
            },
        );
        assert!(!h.export_collection_consistent());
    }
    #[test]
    fn export_frames_exist_only_in_the_signed_fixture_profile() {
        let (dir, mut h) = host();
        h.journal.job.coding.as_mut().unwrap().schema = PROFILE.into();
        let mut f = two_files()[0].clone();
        f["schema"] = json!(PROFILE);
        assert!(advance(&mut h, &[f]).is_err());
        assert!(!dir.path().join("exports/a.json").exists());
    }
    #[test]
    fn export_before_readiness_is_refused() {
        let (dir, mut h) = host();
        h.journal.coding.as_mut().unwrap().ready_observed = false;
        assert!(advance(&mut h, &two_files()[..1]).is_err());
        assert!(!dir.path().join("exports/a.json").exists());
    }
    #[test]
    fn tmpfs_replaces_the_export_bind_for_the_signed_fixture_only() {
        let (_d, mut h) = host();
        h.journal.container_name = "we-pi-test".into();
        let signed = h.coding_inference_args();
        assert!(
            signed
                .windows(2)
                .any(|w| w[0] == "--tmpfs" && w[1] == TMPFS)
        );
        assert!(!signed.iter().any(|a| a.contains("dst=/export")));
        h.journal.job.coding.as_mut().unwrap().schema = PROFILE.into();
        let other = h.coding_inference_args();
        assert!(!other.iter().any(|a| a == TMPFS));
        assert!(other.iter().any(|a| a.contains("dst=/export")));
    }
    #[test]
    fn container_flags_keep_every_confinement_flag() {
        let (_d, h) = host();
        let args = h.coding_inference_args();
        for pair in [
            ["--network", "none"],
            ["--cap-drop", "ALL"],
            ["--security-opt", "no-new-privileges"],
            ["--pids-limit", "64"],
            ["--memory", "512m"],
            ["--cpus", "1"],
            ["--user", "1000:1000"],
            ["--pull", "never"],
        ] {
            assert!(
                args.windows(2).any(|w| w[0] == pair[0] && w[1] == pair[1]),
                "{pair:?}"
            );
        }
        assert!(args.iter().any(|a| a == "--read-only"));
        assert!(
            !args
                .iter()
                .any(|a| a.contains("docker.sock") || a == "--privileged")
        );
    }
    /// The real shim text (its `emit` and `exportBundle`, run under node against a temp directory)
    /// produces frames the Host accepts, so the two sides cannot drift apart unnoticed.
    #[test]
    fn real_shim_export_frames_are_accepted_by_the_host() {
        let shim = std::str::from_utf8(SHIM).unwrap();
        let cut = |from: &str, to: &str| {
            let a = shim.find(from).unwrap_or_else(|| panic!("{from}"));
            let b = a + shim[a..].find(to).unwrap_or_else(|| panic!("{to}"));
            shim[a..b].to_string()
        };
        let emit = cut("const emit=value=>{", "\n};\n") + "\n};\n";
        let bundle = cut("const exportBundle=()=>{", "\nlet startResolve;");
        let (dir, mut h) = host();
        let src = tempfile::tempdir().unwrap();
        fs::create_dir(src.path().join("sessions")).unwrap();
        fs::write(
            src.path().join("coding-native-history.json"),
            b"{\"messages\":[1]}\n",
        )
        .unwrap();
        let big: Vec<u8> = (0..60_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(
            src.path()
                .join("sessions/2026-11-13T05-50-00-000Z_abc.jsonl"),
            &big,
        )
        .unwrap();
        fs::write(src.path().join("empty"), b"").unwrap();
        let script = format!(
            "import {{createHash}} from 'node:crypto';import {{readdirSync,lstatSync,readFileSync}} from 'node:fs';\n\
             const fixtureSigned=true,schema='{SIGNED_PROFILE}',FRAME_BYTES=65536;let session;const manager={{getSessionId:()=>'{SESSION}'}};\n{emit}\n{}\nexportBundle();\n",
            bundle.replace(
                "walk('/export'",
                &format!("walk('{}'", src.path().display())
            )
        );
        let script_path = dir.path().join("shim-export.mjs");
        fs::write(&script_path, script).unwrap();
        let out = std::process::Command::new("node")
            .arg(&script_path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lines = out
            .stdout
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .count();
        assert!(lines >= 5, "{lines}");
        h.coding_advance(&out.stdout).unwrap();
        assert!(h.export_finalized());
        let e = h.journal.coding.as_ref().unwrap().export.as_ref().unwrap();
        assert_eq!(e.files.len(), 3);
        assert_eq!(e.total_bytes, 17 + 60_000);
        assert_eq!(
            fs::read(
                dir.path()
                    .join("exports/sessions/2026-11-13T05-50-00-000Z_abc.jsonl")
            )
            .unwrap(),
            big
        );
    }
    /// Frames captured from a real Host run (attempt 1 stdout; the container, SDK and shim were real).
    fn real_frames() -> Vec<Value> {
        include_str!("../testdata/real-export-frames.jsonl")
            .lines()
            .map(|l| {
                let mut v: Value = serde_json::from_str(l).unwrap();
                v["sessionId"] = json!(SESSION);
                v
            })
            .collect()
    }
    #[test]
    fn real_session_file_names_fit_the_path_bounds() {
        let name = "sessions/2026-11-13T05-54-50-646Z_01a1d379-1695-7511-9628-93c4445f0848.jsonl";
        assert!(valid_path(name));
        assert!(valid_path("coding-native-history.json"));
    }
    #[test]
    fn frames_captured_from_a_real_run_are_accepted_and_complete_the_run() {
        let frames = real_frames();
        assert_eq!(frames.len(), 3);
        let (dir, mut h) = complete_ready_host();
        let mut all = frames.clone();
        all.push(complete_frame());
        advance(&mut h, &all).unwrap();
        assert!(h.export_finalized());
        assert!(h.journal.coding.as_ref().unwrap().completed);
        let e = h.journal.coding.as_ref().unwrap().export.as_ref().unwrap();
        assert_eq!((e.files.len(), e.total_bytes), (2, 19_724));
        for (p, f) in &e.files {
            assert_eq!(
                file_digest(&dir.path().join("exports").join(p), None).unwrap(),
                f.sha256
            );
        }
        // Dropping any one captured frame, or swapping the two parts, is not accepted.
        for broken in [
            vec![frames[0].clone(), frames[2].clone()],
            vec![frames[1].clone(), frames[0].clone(), frames[2].clone()],
            vec![frames[0].clone(), frames[1].clone()],
        ] {
            let (_d, mut h) = complete_ready_host();
            let mut all = broken;
            all.push(complete_frame());
            assert!(advance(&mut h, &all).is_err());
            assert!(!h.journal.coding.as_ref().unwrap().completed);
        }
    }
}
