//! Restrictive, owner-installed M3a read profile and immutable object-source adapter.
use crate::{CannotRead, Error};
use gix::{hash::ObjectId, objs::Kind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadProfile {
    pub text_bytes: usize,
    pub text_lines: usize,
    pub payload_bytes: usize,
    pub decisions: usize,
    pub source_bytes: usize,
    pub index_bytes: usize,
    pub packs: usize,
    pub depth: usize,
    pub steps: usize,
    pub objects: usize,
}
impl Default for ReadProfile {
    fn default() -> Self {
        Self {
            text_bytes: 1 << 20,
            text_lines: 10000,
            payload_bytes: 8 << 20,
            decisions: 16,
            source_bytes: 512 << 20,
            index_bytes: 32 << 20,
            packs: 16,
            depth: 64,
            steps: 32768,
            objects: 8192,
        }
    }
}
impl ReadProfile {
    pub fn validate(&self) -> Result<(), Error> {
        let d = Self::default();
        if [
            self.text_bytes,
            self.text_lines,
            self.payload_bytes,
            self.decisions,
            self.source_bytes,
            self.index_bytes,
            self.packs,
            self.depth,
            self.steps,
            self.objects,
        ]
        .contains(&0)
            || self.text_bytes > d.text_bytes
            || self.text_lines > d.text_lines
            || self.payload_bytes > d.payload_bytes
            || self.decisions > d.decisions
            || self.source_bytes > d.source_bytes
            || self.index_bytes > d.index_bytes
            || self.packs > d.packs
            || self.depth > d.depth
            || self.steps > d.steps
            || self.objects > d.objects
        {
            return Err(refusal(
                "Profile",
                "limits must be positive and no larger than profile v1",
            ));
        }
        Ok(())
    }
}
pub(crate) fn refusal(kind: &str, reason: &str) -> Error {
    CannotRead::ObjectStore {
        id: "bounded-reader".into(),
        reason: format!("{kind}: {reason}"),
    }
    .into()
}
#[derive(Clone)]
struct Decoded {
    kind: Kind,
    bytes: Vec<u8>,
}
struct Pack {
    index: gix_pack::index::File<Vec<u8>>,
    path: PathBuf,
    data: Option<gix_pack::data::File<Vec<u8>>>,
    offsets: BTreeMap<u64, ObjectId>,
    digest: String,
    metadata: usize,
}
pub(crate) struct BoundedObjects {
    pub profile: ReadProfile,
    root: PathBuf,
    packs: Vec<Pack>,
    cache: HashMap<ObjectId, Decoded>,
    active: HashSet<ObjectId>,
    source: usize,
    payload: usize,
    accounted: HashSet<ObjectId>,
    steps: usize,
    metadata_seen: HashSet<(PathBuf, String)>,
    metadata_total: usize,
    origins: HashMap<ObjectId, Vec<PathBuf>>,
}
fn fail(e: impl std::fmt::Display) -> Error {
    refusal("SourceUnavailable", &e.to_string())
}
fn copy(path: &Path, remaining: usize) -> Result<Vec<u8>, Error> {
    let mut f = File::open(path).map_err(fail)?;
    let n = usize::try_from(f.metadata().map_err(fail)?.len()).map_err(fail)?;
    if n > remaining {
        return Err(refusal("SourceLimit", "declared physical source bytes"));
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(n).map_err(fail)?;
    bytes.resize(n, 0);
    f.read_exact(&mut bytes).map_err(fail)?;
    let mut extra = [0];
    if f.read(&mut extra).map_err(fail)? != 0 {
        return Err(refusal(
            "SourceChanged",
            "source length changed while copying",
        ));
    }
    Ok(bytes)
}
impl BoundedObjects {
    pub fn new(root: PathBuf, profile: ReadProfile) -> Result<Self, Error> {
        profile.validate()?;
        // Alternates and MIDX require a separately admitted source inventory. Refuse rather than fall through.
        if root.join("info/alternates").exists() || root.join("pack/multi-pack-index").exists() {
            return Err(refusal(
                "ScopeUnavailable",
                "alternates and multi-pack indices are unsupported",
            ));
        }
        let mut paths = Vec::new();
        let mut inventory_steps = 0usize;
        if root.join("pack").exists() {
            for entry in std::fs::read_dir(root.join("pack")).map_err(fail)? {
                inventory_steps += 1;
                if inventory_steps > profile.steps {
                    return Err(refusal("TraversalLimit", "pack directory inventory"));
                }
                let p = entry.map_err(fail)?.path();
                if p.extension().is_some_and(|e| e == "idx") {
                    paths.push(p);
                    if paths.len() > profile.packs {
                        return Err(refusal("MetadataLimit", "pack inventory"));
                    }
                }
            }
        }
        paths.sort();
        let mut packs = Vec::new();
        let mut metadata = 0usize;
        for path in paths {
            let bytes = copy(&path, profile.index_bytes.saturating_sub(metadata))?;
            metadata += bytes.len();
            let index_len = bytes.len();
            let digest = format!("{:x}", Sha256::digest(&bytes));
            let index =
                gix_pack::index::File::from_data(bytes, path.clone(), gix::hash::Kind::Sha1)
                    .map_err(fail)?;
            let extra = (index.num_objects() as usize)
                .checked_mul(48)
                .ok_or_else(|| refusal("MetadataLimit", "offset inventory overflow"))?;
            metadata = metadata
                .checked_add(extra)
                .ok_or_else(|| refusal("MetadataLimit", "index inventory overflow"))?;
            if metadata > profile.index_bytes {
                return Err(refusal(
                    "MetadataLimit",
                    "index and reverse-offset inventory",
                ));
            }
            let mut offsets = BTreeMap::new();
            for i in 0..index.num_objects() {
                offsets.insert(
                    index.pack_offset_at_index(i),
                    index.oid_at_index(i).to_owned(),
                );
            }
            packs.push(Pack {
                index,
                path: path.with_extension("pack"),
                data: None,
                offsets,
                digest,
                metadata: index_len + extra,
            });
        }
        let metadata_seen = packs
            .iter()
            .map(|p| (p.path.clone(), p.digest.clone()))
            .collect();
        Ok(Self {
            profile,
            root,
            packs,
            cache: HashMap::new(),
            active: HashSet::new(),
            source: 0,
            payload: 0,
            accounted: HashSet::new(),
            steps: inventory_steps,
            metadata_seen,
            metadata_total: metadata,
            origins: HashMap::new(),
        })
    }
    pub fn prefix(&mut self, hex: &str) -> Result<HashSet<ObjectId>, Error> {
        // Prefix uniqueness is observed anew; immutable payload custody is not a frozen universe.
        let mut fresh = Self::new(self.root.clone(), self.profile.clone())?;
        self.steps = self
            .steps
            .checked_add(fresh.steps)
            .ok_or_else(|| refusal("TraversalLimit", "inventory overflow"))?;
        if self.steps > self.profile.steps {
            return Err(refusal("TraversalLimit", "live pack inventory"));
        }
        for pack in &mut fresh.packs {
            let key = (pack.path.clone(), pack.digest.clone());
            if self.metadata_seen.insert(key) {
                self.metadata_total = self
                    .metadata_total
                    .checked_add(pack.metadata)
                    .ok_or_else(|| refusal("MetadataLimit", "metadata overflow"))?;
            }
            if self.metadata_total > self.profile.index_bytes {
                return Err(refusal("MetadataLimit", "distinct index generations"));
            }
            if let Some(old) = self
                .packs
                .iter_mut()
                .find(|p| p.path == pack.path && p.digest == pack.digest)
            {
                pack.data = old.data.take();
            }
        }
        self.packs = fresh.packs;
        let mut candidates = HashSet::new();
        for pack in &self.packs {
            for i in 0..pack.index.num_objects() {
                let id = pack.index.oid_at_index(i);
                if id.to_string().starts_with(hex) {
                    candidates.insert(id.to_owned());
                    if candidates.len() > self.profile.objects {
                        return Err(refusal("MetadataLimit", "prefix candidates"));
                    }
                }
            }
        }
        let directory = self.root.join(&hex[..2]);
        if directory.exists() {
            for entry in std::fs::read_dir(directory).map_err(fail)? {
                self.steps += 1;
                if self.steps > self.profile.steps {
                    return Err(refusal("TraversalLimit", "loose prefix inventory"));
                }
                let name = entry.map_err(fail)?.file_name();
                if let Some(name) = name.to_str() {
                    let full = format!("{}{name}", &hex[..2]);
                    if full.len() == 40
                        && full.starts_with(hex)
                        && let Ok(id) = ObjectId::from_hex(full.as_bytes())
                    {
                        candidates.insert(id);
                    }
                }
            }
        }
        Ok(candidates)
    }
    pub(crate) fn copies(&self) -> Vec<(ObjectId, Kind, Vec<u8>)> {
        self.cache
            .iter()
            .map(|(id, d)| (*id, d.kind, d.bytes.clone()))
            .collect()
    }
    pub fn accounting(&self) -> serde_json::Value {
        serde_json::json!({"source_copy_bytes":self.source,"distinct_payload_bytes":self.payload,"index_metadata_bytes":self.metadata_total,"traversal_steps":self.steps,"distinct_objects":self.cache.len(),"copied_packs":self.packs.iter().filter(|p|p.data.is_some()).count()})
    }
    pub fn read(&mut self, id: ObjectId) -> Result<(Kind, Vec<u8>), Error> {
        let d = self.inner(id, 0)?;
        Ok((d.kind, d.bytes))
    }
    fn inner(&mut self, id: ObjectId, depth: usize) -> Result<Decoded, Error> {
        self.inner_at(id, depth, None)
    }
    fn inner_at(
        &mut self,
        id: ObjectId,
        depth: usize,
        location: Option<(usize, u64)>,
    ) -> Result<Decoded, Error> {
        self.steps += 1;
        if self.steps > self.profile.steps {
            return Err(refusal("TraversalLimit", "object/dependency traversal"));
        }
        if let Some(paths) = self.origins.get(&id) {
            for path in paths {
                if !path.is_file() {
                    return Err(CannotRead::ObjectUnavailable { id: id.to_string() }.into());
                }
            }
        }
        if location.is_none()
            && let Some(d) = self.cache.get(&id)
        {
            return Ok(d.clone());
        }
        if depth > self.profile.depth {
            return Err(refusal("DependencyLimit", "delta depth"));
        }
        if !self.cache.contains_key(&id)
            && self.cache.len()
                + self
                    .active
                    .iter()
                    .filter(|id| !self.cache.contains_key(*id))
                    .count()
                >= self.profile.objects
        {
            return Err(refusal("MetadataLimit", "distinct decoded object count"));
        }
        if !self.active.insert(id) {
            return Err(refusal("DependencyLimit", "delta cycle"));
        }
        let result = self.decode(id, depth, location);
        self.active.remove(&id);
        let d = result?;
        super::objects::verify_hash(id, d.kind, &d.bytes)?;
        if d.kind == Kind::Blob
            && (d.bytes.len() > self.profile.text_bytes
                || d.bytes.iter().filter(|b| **b == b'\n').count()
                    + usize::from(!d.bytes.is_empty() && !d.bytes.ends_with(b"\n"))
                    > self.profile.text_lines)
        {
            return Err(refusal("TextLimit", "blob bytes or lines"));
        }
        self.cache.insert(id, d.clone());
        Ok(d)
    }
    fn admit(&mut self, id: ObjectId, n: u64) -> Result<usize, Error> {
        let n = usize::try_from(n).map_err(fail)?;
        if n > self.profile.text_bytes {
            return Err(refusal(
                "PayloadLimit",
                "declared object size before allocation",
            ));
        }
        if !self.accounted.insert(id) {
            return Ok(n);
        }
        self.payload = self
            .payload
            .checked_add(n)
            .ok_or_else(|| refusal("PayloadLimit", "aggregate overflow"))?;
        if self.payload > self.profile.payload_bytes {
            return Err(refusal("PayloadLimit", "aggregate distinct decoded bytes"));
        }
        Ok(n)
    }
    fn decode(
        &mut self,
        id: ObjectId,
        depth: usize,
        location: Option<(usize, u64)>,
    ) -> Result<Decoded, Error> {
        let found = location.or_else(|| {
            self.packs.iter().enumerate().find_map(|(p, pack)| {
                pack.index
                    .lookup(id)
                    .map(|i| (p, pack.index.pack_offset_at_index(i)))
            })
        });
        let Some((pi, offset)) = found else {
            let hex = id.to_string();
            let origin = self.root.join(&hex[..2]).join(&hex[2..]);
            self.origins.insert(id, vec![origin.clone()]);
            let bytes = copy(
                &origin,
                self.profile.source_bytes.saturating_sub(self.source),
            )?;
            self.source += bytes.len();
            let mut rd = bytes.as_slice();
            let mut state = gix::zlib::Decompress::default();
            let mut header = Vec::new();
            loop {
                let mut b = [0];
                if gix::zlib::stream::inflate::read(&mut rd, &mut state, &mut b).map_err(fail)? != 1
                {
                    return Err(refusal("Malformed", "truncated loose header"));
                }
                if b[0] == 0 {
                    break;
                }
                header.push(b[0]);
                if header.len() > 64 {
                    return Err(refusal("MetadataLimit", "loose header"));
                }
            }
            let header = std::str::from_utf8(&header).map_err(fail)?;
            let (k, n) = header
                .split_once(' ')
                .ok_or_else(|| refusal("Malformed", "loose header"))?;
            let kind = match k {
                "blob" => Kind::Blob,
                "tree" => Kind::Tree,
                "commit" => Kind::Commit,
                "tag" => Kind::Tag,
                _ => return Err(refusal("Malformed", "loose kind")),
            };
            let n = self.admit(id, n.parse().map_err(fail)?)?;
            let mut out = vec![0; n];
            if gix::zlib::stream::inflate::read(&mut rd, &mut state, &mut out).map_err(fail)? != n {
                return Err(refusal("Malformed", "loose size"));
            }
            let mut overflow = [0];
            if gix::zlib::stream::inflate::read(&mut rd, &mut state, &mut overflow).map_err(fail)?
                != 0
            {
                return Err(refusal("PayloadLimit", "loose expansion"));
            }
            if state
                .decompress(&[], &mut overflow, gix::zlib::FlushDecompress::Finish)
                .map_err(fail)?
                != gix::zlib::Status::StreamEnd
                || state.total_in() != bytes.len() as u64
            {
                return Err(refusal("Malformed", "loose stream not complete"));
            }
            return Ok(Decoded { kind, bytes: out });
        };
        self.origins.insert(
            id,
            vec![
                self.packs[pi].path.clone(),
                self.packs[pi].path.with_extension("idx"),
            ],
        );
        if !self.packs[pi].path.is_file() || !self.packs[pi].path.with_extension("idx").is_file() {
            return Err(CannotRead::ObjectUnavailable { id: id.to_string() }.into());
        }
        if self.packs[pi].data.is_none() {
            let bytes = copy(
                &self.packs[pi].path,
                self.profile.source_bytes.saturating_sub(self.source),
            )?;
            self.source += bytes.len();
            self.packs[pi].data = Some(
                gix_pack::data::File::from_data(
                    bytes,
                    self.packs[pi].path.clone(),
                    gix::hash::Kind::Sha1,
                )
                .map_err(fail)?
                .with_alloc_limit_bytes(Some(self.profile.payload_bytes)),
            );
        }
        let entry = self.packs[pi]
            .data
            .as_ref()
            .unwrap()
            .entry(offset)
            .map_err(fail)?;
        if entry.decompressed_size > self.profile.text_bytes as u64 {
            return Err(refusal(
                "PayloadLimit",
                "declared object size before allocation",
            ));
        }
        use gix_pack::data::entry::Header;
        // OFS and same-pack REF dependencies must follow the physical chain
        // the decoder consumes, even when another pack or the OID cache has a copy.
        let base = match entry.header {
            Header::OfsDelta { base_distance } => {
                let off = entry
                    .checked_base_pack_offset(base_distance)
                    .ok_or_else(|| refusal("Malformed", "OFS underflow"))?;
                let base = *self.packs[pi].offsets.get(&off).ok_or_else(|| {
                    refusal(
                        "ScopeUnavailable",
                        "OFS base has no admitted index identity",
                    )
                })?;
                Some((base, Some((pi, off))))
            }
            Header::RefDelta { base_id } => Some((
                base_id,
                self.packs[pi]
                    .index
                    .lookup(base_id)
                    .map(|i| (pi, self.packs[pi].index.pack_offset_at_index(i))),
            )),
            _ => None,
        };
        if let Some((base, location)) = base {
            self.inner_at(base, depth + 1, location)?;
        }
        let pack = self.packs[pi].data.as_ref().unwrap();
        let mut inflate = gix::zlib::Inflate::default();
        let hdr = pack
            .decode_header(entry.clone(), &mut inflate, &|base| {
                Ok(self.packs[pi]
                    .index
                    .lookup(base)
                    .map(|i| pack.entry(self.packs[pi].index.pack_offset_at_index(i)))
                    .transpose()?
                    .map(gix_pack::data::decode::header::ResolvedBase::InPack)
                    .or_else(|| {
                        self.cache.get(base).map(|d| {
                            gix_pack::data::decode::header::ResolvedBase::OutOfPack {
                                kind: d.kind,
                                num_deltas: None,
                            }
                        })
                    }))
            })
            .map_err(fail)?;
        self.admit(id, hdr.object_size)?;
        let pack = self.packs[pi].data.as_ref().unwrap();
        let mut out = Vec::new();
        let result = pack
            .decode_entry(
                entry,
                &mut out,
                &mut inflate,
                &|base, buf| {
                    if let Some(d) = self.cache.get(base) {
                        buf.clear();
                        buf.extend_from_slice(&d.bytes);
                        Ok(Some(
                            gix_pack::data::decode::entry::ResolvedBase::OutOfPack {
                                kind: d.kind,
                                end: buf.len(),
                            },
                        ))
                    } else {
                        Err(gix::error::validation("unadmitted delta base").into())
                    }
                },
                &mut gix_pack::cache::Never,
            )
            .map_err(fail)?;
        if out.len() as u64 != hdr.object_size {
            return Err(refusal("Malformed", "decoded size differs from admission"));
        }
        Ok(Decoded {
            kind: result.kind,
            bytes: out,
        })
    }
}
