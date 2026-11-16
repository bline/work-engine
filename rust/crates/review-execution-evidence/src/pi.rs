//! Private, original-A checked native Pi snapshots. This branch grants no
//! campaign/provider authority. A live store cannot be reconstructed from a DTO.
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
pub type Result<T> = std::result::Result<T, &'static str>;
pub const MAX_SNAPSHOT: usize = 2 * 1024 * 1024;
pub const MAX_STORE: usize = 16 * 1024 * 1024;
const PROFILE: &str = "pi-native-inmemory-sse-v1";
fn clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(u64::MAX)
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn io<T>(r: std::io::Result<T>) -> Result<T> {
    r.map_err(|_| "pi_store_io")
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub session: String,
    pub attempt: String,
    pub generation: u64,
    pub profile: String,
    #[serde(default)]
    pub admission: String,
    #[serde(default)]
    pub owner_profile: String,
}
impl Binding {
    pub fn native(session: String, attempt: String, generation: u64) -> Self {
        Self {
            session,
            attempt,
            generation,
            profile: PROFILE.into(),
            admission: String::new(),
            owner_profile: String::new(),
        }
    }
}
// Construction is private and serialization deliberately absent.
pub struct Snapshot {
    ordinal: u64,
    digest: String,
    binding: Binding,
    header: Value,
    entries: Vec<Value>,
    leaf: String,
}
impl Snapshot {
    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn header(&self) -> &Value {
        &self.header
    }
    pub fn entries(&self) -> &[Value] {
        &self.entries
    }
    pub fn leaf(&self) -> &str {
        &self.leaf
    }
}
/// Retention sequencing only; never a transport permit.
pub struct Pending {
    sequence: u64,
    predecessor: Option<String>,
    binding: Binding,
    identity: (u64, u64),
}
pub struct LiveStore {
    dir: File,
    original: Vec<u8>,
    binding: Binding,
    expires: u64,
    next: u64,
    head: Option<String>,
    last: Option<Snapshot>,
    meter: std::sync::Arc<std::sync::Mutex<Meter>>,
    pending: Option<u64>,
    root: std::path::PathBuf,
    identity: (u64, u64),
}
#[derive(Default)]
struct Meter {
    charge: usize,
    poisoned: bool,
    registry_created: bool,
    registry: Option<RegistrySeal>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrySeal {
    directory: String,
    identity: (u64, u64),
    generation: u64,
    profile: String,
    image: String,
    metadata: String,
}
mod grammar;
mod registry;
pub use registry::{CheckedRegistry, RegistryConnection};
// Bounded native representation grammar: JSON values; strings beginning with
// JSON container/quote syntax are strict JSON or strict JSONL (all records).
// Explicit base64:/base64url: productions require canonical decoding. Canonical
// bare standard/URL-safe encodings are also recursively interpreted. Other UTF8
// strings and decoded binary are leaves. Every layer includes keys and names.
// Bounds apply to the whole inspection; no prefix-only success exists.
pub const MAX_DEPTH: usize = 32;
pub const MAX_RECORDS: usize = 256;
pub const MAX_INSPECTED: usize = 8 * 1024 * 1024;
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("duplicate-free JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut a: S,
            ) -> std::result::Result<Strict, S::Error> {
                let mut v = vec![];
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0);
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut a: M,
            ) -> std::result::Result<Strict, M::Error> {
                let mut v = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate"));
                    }
                    let x = a.next_value::<Strict>()?;
                    v.insert(k, x.0);
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn parse_json(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_SNAPSHOT {
        return Err("pi_snapshot_bound");
    }
    serde_json::from_slice::<Strict>(bytes)
        .map(|v| v.0)
        .map_err(|_| "pi_strict_json")
}
struct Inspection<'a> {
    a: &'a [u8],
    bytes: usize,
    records: usize,
    work: usize,
}
impl Inspection<'_> {
    fn budget(&mut self, n: usize, depth: usize) -> Result<()> {
        self.bytes = self.bytes.checked_add(n).ok_or("pi_inspect_bound")?;
        self.work += 1;
        if depth > MAX_DEPTH || self.bytes > MAX_INSPECTED || self.work > 65536 {
            return Err("pi_inspect_bound");
        }
        Ok(())
    }
    fn text(&mut self, s: &str, depth: usize) -> Result<()> {
        self.budget(s.len(), depth)?;
        check_bytes(s.as_bytes(), self.a)?;
        let trimmed = s.trim();
        if matches!(trimmed.as_bytes().first(), Some(b'{' | b'[' | b'"')) {
            // Parse the entire JSON first. Multirecord interpretation is allowed only
            // when every nonempty line is a complete strict JSON production.
            if let Ok(v) = parse_json(trimmed.as_bytes()) {
                return self.value(&v, depth + 1);
            }
            let lines: Vec<_> = trimmed.lines().collect();
            if lines.len() < 2 || lines.len() > MAX_RECORDS {
                return Err("pi_structured_text");
            }
            for line in lines {
                self.records += 1;
                if self.records > MAX_RECORDS {
                    return Err("pi_records");
                }
                let v = parse_json(line.as_bytes())?;
                self.value(&v, depth + 1)?;
            }
            return Ok(());
        }
        for (prefix, engine) in [("base64:", STANDARD), ("base64url:", URL_SAFE_NO_PAD)] {
            if let Some(body) = trimmed.strip_prefix(prefix) {
                let b = engine.decode(body).map_err(|_| "pi_declared_encoding")?;
                if engine.encode(&b) != body {
                    return Err("pi_declared_encoding");
                }
                return self.decoded(&b, depth + 1);
            }
        }
        // A canonical complete bare encoding is a grammar production, including
        // short encodings. Failed decodings are ordinary text, never declared data.
        if !trimmed.is_empty() {
            for engine in [STANDARD, URL_SAFE_NO_PAD] {
                if let Ok(b) = engine.decode(trimmed) {
                    if !b.is_empty() && engine.encode(&b) == trimmed {
                        self.decoded(&b, depth + 1)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn decoded(&mut self, b: &[u8], depth: usize) -> Result<()> {
        self.budget(b.len(), depth)?;
        check_bytes(b, self.a)?;
        if let Ok(s) = std::str::from_utf8(b) {
            self.text(s, depth + 1)?;
        }
        Ok(())
    }
    fn value(&mut self, v: &Value, depth: usize) -> Result<()> {
        self.budget(1, depth)?;
        match v {
            Value::String(s) => self.text(s, depth + 1)?,
            Value::Array(xs) => {
                for x in xs {
                    self.value(x, depth + 1)?;
                }
            }
            Value::Object(m) => {
                let declared = m.contains_key("$encoding") || m.contains_key("encoding");
                if let Some(encoding) = m.get("$encoding").or_else(|| m.get("encoding")) {
                    let encoding = encoding.as_str().ok_or("pi_declared_encoding")?;
                    let data = m
                        .get("data")
                        .and_then(Value::as_str)
                        .ok_or("pi_declared_encoding")?;
                    // Inspect the exact raw representation as well as its declared
                    // interpretation, but do not count the same JSONL records a
                    // second time through the generic string production below.
                    self.budget(data.len(), depth + 1)?;
                    check_bytes(data.as_bytes(), self.a)?;
                    match encoding {
                        "base64" => self.text(&format!("base64:{data}"), depth + 1)?,
                        "base64url" => self.text(&format!("base64url:{data}"), depth + 1)?,
                        "json" => {
                            let v = parse_json(data.as_bytes())?;
                            self.value(&v, depth + 1)?;
                        }
                        "jsonl" => {
                            if data.lines().count() > MAX_RECORDS {
                                return Err("pi_records");
                            }
                            for line in data.lines() {
                                self.records += 1;
                                if self.records > MAX_RECORDS {
                                    return Err("pi_records");
                                }
                                let v = parse_json(line.as_bytes())?;
                                self.value(&v, depth + 1)?;
                            }
                        }
                        "utf8" => self.text(data, depth + 1)?,
                        _ => return Err("pi_declared_encoding"),
                    }
                }
                for (k, v) in m {
                    self.text(k, depth + 1)?;
                    if !(declared && k == "data") {
                        self.value(v, depth + 1)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
fn check_value(v: &Value, a: &[u8], depth: usize) -> Result<()> {
    Inspection {
        a,
        bytes: 0,
        records: 0,
        work: 0,
    }
    .value(v, depth)
}
/// Longest suffix of `b` that is a proper, non-empty prefix of `a`.
pub fn prefix_suffix(b: &[u8], a: &[u8]) -> usize {
    let max = b.len().min(a.len().saturating_sub(1));
    (1..=max)
        .rev()
        .find(|&n| b[b.len() - n..] == a[..n])
        .unwrap_or(0)
}
fn check_bytes(b: &[u8], a: &[u8]) -> Result<()> {
    if a.is_empty() || b.windows(a.len()).any(|w| w == a) {
        Err("pi_original_a_exclusion")
    } else {
        Ok(())
    }
}
fn timestamp(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).is_ok()
    })
}
fn validate(binding: &Binding, header: &Value, entries: &[Value], leaf: &str) -> Result<()> {
    if binding.profile != PROFILE
        || binding.session.len() != 36
        || binding.generation == 0
        || binding.attempt.is_empty()
        || header.as_object().is_none_or(|m| {
            m.keys().any(|k| {
                !matches!(
                    k.as_str(),
                    "type" | "version" | "id" | "cwd" | "timestamp" | "parentSession"
                )
            })
        })
        || header["type"] != "session"
        || header["version"] != 3
        || header["id"] != binding.session
        || header["cwd"] != "/scratch"
        || header.get("parentSession").is_some_and(|v| !v.is_string())
        || !timestamp(&header["timestamp"])
        || entries.is_empty()
        || entries.len() > 256
    {
        return Err("pi_native_shape");
    }
    let mut ids = BTreeSet::new();
    let mut tool_calls = std::collections::BTreeMap::new();
    let mut prior = None;
    for e in entries {
        let id = e["id"].as_str().ok_or("pi_entry_id")?;
        let parent = e.get("parentId").ok_or("pi_explicit_parent")?;
        if id.is_empty()
            || !ids.insert(id)
            || !timestamp(&e["timestamp"])
            || match prior {
                None => !parent.is_null(),
                Some(p) => parent.as_str() != Some(p),
            }
        {
            return Err("pi_native_chain");
        }
        let allowed = match e["type"].as_str() {
            Some("model_change") => {
                &["type", "id", "parentId", "timestamp", "provider", "modelId"][..]
            }
            Some("thinking_level_change") => {
                &["type", "id", "parentId", "timestamp", "thinkingLevel"][..]
            }
            Some("message") => &["type", "id", "parentId", "timestamp", "message"][..],
            _ => return Err("pi_entry_kind"),
        };
        if e.as_object()
            .is_none_or(|m| m.keys().any(|k| !allowed.contains(&k.as_str())))
        {
            return Err("pi_entry_fields");
        }
        match e["type"].as_str() {
            Some("model_change")
                if e["provider"] == "openai-codex" && e["modelId"] == "gpt-6-luna" => {}
            Some("thinking_level_change") if e["thinkingLevel"] == "off" => {}
            Some("message") => {
                grammar::message(&e["message"], &mut tool_calls)?;
            }
            _ => return Err("pi_entry_kind"),
        };
        prior = Some(id);
    }
    if prior != Some(leaf) {
        return Err("pi_native_leaf");
    }
    Ok(())
}
fn create(dir: &File, name: &str, b: &[u8]) -> Result<()> {
    let name = CString::new(name).map_err(|_| "pi_name")?;
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err("pi_create_only");
    };
    let mut f = unsafe { File::from_raw_fd(fd) };
    io(f.write_all(b))?;
    io(f.sync_all())?;
    io(dir.sync_all())
}
// Bootstrap and nested directory creation consume the checked destination here.
// The held parent descriptor owns the actual mkdirat destination; callers never
// obtain a reusable permission separate from the retaining operation.
fn checked_directory(path: &Path, original: &[u8], parent: Option<&File>) -> Result<File> {
    let mut inspect = Inspection {
        a: original,
        bytes: 0,
        records: 0,
        work: 0,
    };
    inspect.text(path.to_str().ok_or("pi_root")?, 0)?;
    for component in path.components() {
        inspect.text(component.as_os_str().to_str().ok_or("pi_name")?, 0)?;
    }
    let name = path.file_name().and_then(|s| s.to_str()).ok_or("pi_name")?;
    let name = CString::new(name).map_err(|_| "pi_name")?;
    let owned;
    let parent = match parent {
        Some(f) => f,
        None => {
            owned = io(OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path.parent().ok_or("pi_parent")?))?;
            &owned
        }
    };
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        return Err("pi_create_directory");
    }
    io(parent.sync_all())?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err("pi_directory_open");
    }
    let dir = unsafe { File::from_raw_fd(fd) };
    if unsafe { libc::flock(dir.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("pi_writer_fence");
    }
    Ok(dir)
}
fn read(dir: &File, name: &str) -> Result<Vec<u8>> {
    let name = CString::new(name).map_err(|_| "pi_name")?;
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err("pi_read");
    };
    let f = unsafe { File::from_raw_fd(fd) };
    let md = io(f.metadata())?;
    if !md.is_file() || md.len() > MAX_SNAPSHOT as u64 {
        return Err("pi_file_bound");
    };
    let mut b = vec![];
    io(f.take((MAX_SNAPSHOT + 1) as u64).read_to_end(&mut b))?;
    if b.len() > MAX_SNAPSHOT {
        return Err("pi_file_bound");
    }
    Ok(b)
}
fn open_dir(root: &Path) -> Result<File> {
    let f = io(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root))?;
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("pi_writer_fence");
    }
    Ok(f)
}
impl LiveStore {
    /// Explicit access acquisition. Retains original A for this entire live owner;
    /// there is no token rotation/replacement API and no live reopen API.
    pub fn create(root: &Path, original: Vec<u8>, binding: Binding, expires: u64) -> Result<Self> {
        if original.len() < 16 || original.len() > 4096 {
            return Err("pi_access_shape");
        };
        let bytes = serde_json::to_vec(&binding).map_err(|_| "pi_binding")?;
        check_bytes(&bytes, &original)?;
        check_value(
            &serde_json::to_value(&binding).map_err(|_| "pi_binding")?,
            &original,
            0,
        )?;
        if binding.profile != PROFILE {
            return Err("pi_profile");
        };
        let dir = checked_directory(root, &original, None)?;
        Ok(Self {
            dir: dir.try_clone().map_err(|_| "pi_store_io")?,
            original,
            binding,
            expires,
            next: 1,
            head: None,
            last: None,
            meter: std::sync::Arc::new(std::sync::Mutex::new(Meter {
                charge: root.as_os_str().len(),
                ..Default::default()
            })),
            pending: None,
            root: root.to_owned(),
            identity: {
                let m = io(dir.metadata())?;
                (m.dev(), m.ino())
            },
        })
    }
    pub fn bind_admission(&mut self, operation: &str, profile: &str) -> Result<()> {
        self.current()?;
        if !self.binding.admission.is_empty()
            || !self.binding.owner_profile.is_empty()
            || self.pending.is_some()
            || self.last.is_some()
        {
            return Err("pi_binding_sealed");
        }
        let mut binding = self.binding.clone();
        binding.admission = operation.into();
        binding.owner_profile = profile.into();
        self.check(&serde_json::to_value(&binding).map_err(|_| "pi_binding")?)?;
        self.binding = binding;
        Ok(())
    }
    pub fn check_raw(&self, b: &[u8]) -> Result<()> {
        check_bytes(b, &self.original)
    }
    /// Incremental screening for a relayed byte stream. Refuses any complete
    /// original-A occurrence in `pending` and returns how many trailing bytes
    /// must be withheld because they form a proper prefix of A. Forwarding
    /// only the remainder guarantees no forwarded byte can belong to a later-
    /// completed occurrence, without exposing A or its length to callers.
    pub fn stream_hold(&self, pending: &[u8]) -> Result<usize> {
        check_bytes(pending, &self.original)?;
        Ok(prefix_suffix(pending, &self.original))
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn check(&self, v: &Value) -> Result<Vec<u8>> {
        check_value(v, &self.original, 0)?;
        let b = serde_json::to_vec(v).map_err(|_| "pi_serialize")?;
        if b.len() > MAX_SNAPSHOT {
            return Err("pi_snapshot_bound");
        };
        check_bytes(&b, &self.original)?;
        Ok(b)
    }
    pub fn begin(&mut self, now: u64, sequence: u64, predecessor: Option<&str>) -> Result<Pending> {
        self.current()?;
        if now.max(clock()) >= self.expires
            || self.pending.is_some()
            || sequence != self.next
            || predecessor != self.head.as_deref()
        {
            return Err("pi_dispatch_currentness");
        };
        let b=self.check(&json!({"kind":"intent","binding":self.binding,"sequence":sequence,"predecessor":self.head}))?;
        if self.meter.lock().map_err(|_| "pi_meter")?.charge + b.len() > MAX_STORE {
            return Err("pi_aggregate");
        };
        self.retain(&format!("intent-{sequence:04}.json"), &b, false)?;
        self.pending = Some(sequence);
        Ok(Pending {
            sequence,
            predecessor: self.head.clone(),
            binding: self.binding.clone(),
            identity: self.identity,
        })
    }
    /// Consumption is adjacent to dispatch, under the same exclusive writer lock.
    pub fn consume(&self, grant: Pending, now: u64) -> Result<()> {
        self.current()?;
        if now.max(clock()) >= self.expires
            || self.pending != Some(grant.sequence)
            || self.head != grant.predecessor
            || self.binding != grant.binding
            || self.identity != grant.identity
        {
            return Err("pi_dispatch_expired_or_foreign");
        }
        Ok(())
    }
    pub fn publish(
        &mut self,
        sequence: u64,
        header: Value,
        entries: Vec<Value>,
        leaf: String,
        metadata: Value,
    ) -> Result<&Snapshot> {
        self.current()?;
        if self.pending != Some(sequence) {
            return Err("pi_no_pending_intent");
        };
        validate(&self.binding, &header, &entries, &leaf)?;
        if let Some(last) = &self.last {
            if last.header != header
                || entries.len() <= last.entries.len()
                || !entries.starts_with(&last.entries)
            {
                return Err("pi_evolution_prefix");
            }
        }
        let payload = json!({"schema":PROFILE,"binding":self.binding,"ordinal":sequence,"predecessor":self.head,"header":header,"entries":entries,"leaf":leaf,"metadata":metadata});
        let payload_bytes = self.check(&payload)?;
        let digest = hash(&payload_bytes);
        let b = self.check(&json!({"payload":payload,"sha256":digest}))?;
        if self.meter.lock().map_err(|_| "pi_meter")?.charge + b.len() > MAX_STORE {
            return Err("pi_aggregate");
        };
        self.retain(&format!("snapshot-{sequence:04}.json"), &b, false)?;
        self.head = Some(digest.clone());
        self.next += 1;
        self.pending = None;
        self.last = Some(Snapshot {
            ordinal: sequence,
            digest,
            binding: self.binding.clone(),
            header,
            entries,
            leaf,
        });
        Ok(self.last.as_ref().unwrap())
    }
    pub fn record_control(&mut self, label: &str, event: Value) -> Result<()> {
        if !matches!(
            label,
            "ready"
                | "create-intent"
                | "created"
                | "closure"
                | "closed-observation"
                | "report"
                | "failure"
        ) && !label
            .strip_prefix("stream-")
            .is_some_and(|n| n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("pi_control_label");
        };
        let b = self.check(&json!({"binding":self.binding,"event":event}))?;
        self.retain(
            &format!("control-{label}.json"),
            &b,
            matches!(label, "closure" | "closed-observation" | "report"),
        )
    }
    /// Bounded diagnostic frames share exact-byte exclusion, create-only
    /// publication, destination currentness and aggregate accounting.
    pub fn record_frame(&mut self, ordinal: u64, event: Value) -> Result<()> {
        if ordinal > 128 {
            return Err("pi_frame_ordinal");
        }
        let b = self.check(&json!({"binding":self.binding,"event":event}))?;
        self.retain(&format!("frame-{ordinal:04}.json"), &b, false)
    }
    pub fn dispatch<T>(
        &self,
        grant: Pending,
        now: u64,
        transport: impl FnOnce(&[u8]) -> Result<T>,
    ) -> Result<T> {
        self.consume(grant, now)?;
        transport(&self.original)
    }
    /// Host transport under an already retained pending snapshot intent. The
    /// PiOwner lease remains a separate mandatory authority at each consumer.
    pub fn transport<T>(&self, now: u64, effect: impl FnOnce(&[u8]) -> Result<T>) -> Result<T> {
        self.current()?;
        if self.pending.is_none() || now.max(clock()) >= self.expires {
            return Err("pi_dispatch_currentness");
        }
        effect(&self.original)
    }
    fn current(&self) -> Result<()> {
        let m = io(fs::symlink_metadata(&self.root))?;
        let held = io(self.dir.metadata())?;
        if self.meter.lock().map_err(|_| "pi_meter")?.poisoned
            || !m.is_dir()
            || m.file_type().is_symlink()
            || (m.dev(), m.ino()) != self.identity
            || (held.dev(), held.ino()) != self.identity
        {
            return Err("pi_writer_identity");
        }
        Ok(())
    }
    fn retain(&mut self, name: &str, b: &[u8], closure: bool) -> Result<()> {
        self.current()?;
        Inspection {
            a: &self.original,
            bytes: 0,
            records: 0,
            work: 0,
        }
        .text(name, 0)?;
        let v = parse_json(b)?;
        self.check(&v)?;
        check_bytes(b, &self.original)?;
        let charge = b.len().checked_add(name.len()).ok_or("pi_aggregate")?;
        let limit = if closure {
            MAX_STORE
        } else {
            MAX_STORE - 3 * MAX_SNAPSHOT
        };
        let mut meter = self.meter.lock().map_err(|_| "pi_meter")?;
        if meter.poisoned || meter.charge.checked_add(charge).ok_or("pi_aggregate")? > limit {
            return Err("pi_aggregate");
        }
        // Charge before any retaining operation. Partial writes poison this writer;
        // no capacity is refunded and no subsequent productive publication is safe.
        meter.charge += charge;
        if let Err(e) = create(&self.dir, name, b) {
            meter.poisoned = true;
            return Err(e);
        }
        Ok(())
    }
    /// Exact checked export envelope; only this owner chooses the retained name
    /// and consumes the same serialized artifact bytes it inspected.
    pub fn export(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        self.current()?;
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err("pi_export_name");
        }
        let mut inspect = Inspection {
            a: &self.original,
            bytes: 0,
            records: 0,
            work: 0,
        };
        inspect.text(name, 0)?;
        inspect.decoded(bytes, 0)?;
        let envelope=self.check(&json!({"binding":self.binding,"name":name,"bytes":bytes.len(),"sha256":hash(bytes),"encoding":"base64","data":STANDARD.encode(bytes)}))?;
        self.retain(&format!("export-{name}.json"), &envelope, false)
    }
    pub fn emit_report(&mut self, event: Value) -> Result<()> {
        let b = self.check(&json!({"binding":self.binding,"report":event}))?;
        self.retain("control-report.json", &b, true)?;
        // Exact same final byte object is consumed by the output sink. No caller
        // serialization or unchecked label follows the check.
        io(std::io::stdout().lock().write_all(&b))
    }
    pub fn expected_seal(&self) -> Result<ExpectedSeal> {
        self.current()?;
        let s = self.last.as_ref().ok_or("pi_no_snapshot")?;
        let bytes = read(&self.dir, &format!("snapshot-{:04}.json", s.ordinal))?;
        let publication = hash(&bytes);
        let registry = {
            let meter = self.meter.lock().map_err(|_| "pi_meter")?;
            if meter.poisoned || (meter.registry_created && meter.registry.is_none()) {
                return Err("pi_registry_incomplete");
            }
            meter.registry.clone()
        };
        // Include the derived exact-publication hash and complete custody envelope
        // before it can become anonymous-file retention in another process.
        self.check(&json!({"binding":self.binding,"identity":self.identity,"ordinal":s.ordinal,"digest":s.digest,"publication":publication,"registry":registry}))?;
        Ok(ExpectedSeal {
            binding: self.binding.clone(),
            identity: self.identity,
            ordinal: s.ordinal,
            digest: s.digest.clone(),
            publication,
            registry,
            issuance: Some(self.meter.clone()),
        })
    }
    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }
}
/// Borrowed issuance capability. Its consumer cannot replace serialized bytes,
/// binding, predecessor or exclusion authority between preparation and write.
pub struct ResultPublication<'a> {
    store: &'a mut LiveStore,
    event: Value,
}
impl ResultPublication<'_> {
    /// The authenticated admission owner supplies its held registry connection.
    /// This operation owns exact envelope serialization and the retaining SQL.
    pub fn consume(
        self,
        db: &RegistryConnection,
        operation: &str,
        profile: &str,
        native: &str,
    ) -> Result<()> {
        let s = self.store;
        s.current()?;
        if s.binding.admission != operation
            || s.binding.owner_profile != profile
            || s.binding.session != native
            || s.pending.is_some()
        {
            return Err("pi_result_authority_join");
        }
        let expected = s.expected_seal()?;
        let v = json!({"binding":s.binding,"report":self.event,"seal":{"identity":expected.identity,"ordinal":expected.ordinal,"digest":expected.digest,"publication":expected.publication}});
        let b = s.check(&v)?;
        if !std::sync::Arc::ptr_eq(&s.meter, &db.meter()?) {
            return Err("pi_foreign_registry");
        }
        // The only SQL write is memory-only; the backend consumes/checks the final
        // serialized database publication after the authenticated owner commits.
        let sql = "UPDATE pi_claim SET result_json=?2 WHERE profile_sha=?1 AND operation_id=?3 AND native_id=?4 AND result_json IS NULL";
        let body = std::str::from_utf8(&b).map_err(|_| "pi_result_utf8")?;
        let changed = db
            .execute(sql, rusqlite::params![profile, body, operation, native])
            .map_err(|_| "pi_registry_write");
        match changed {
            Ok(1) => Ok(()),
            _ => {
                s.meter.lock().map_err(|_| "pi_meter")?.poisoned = true;
                Err("pi_result_unresolved")
            }
        }
    }
}
impl LiveStore {
    pub fn result_publication(&mut self, event: Value) -> Result<ResultPublication<'_>> {
        self.current()?;
        Ok(ResultPublication { store: self, event })
    }
}
/// Custody-held independent expectation, issued only by a live store after its
/// exact publication. Not serializable or constructible from caller DTOs.
#[derive(Clone)]
pub struct ExpectedSeal {
    binding: Binding,
    identity: (u64, u64),
    ordinal: u64,
    digest: String,
    publication: String,
    registry: Option<RegistrySeal>,
    issuance: Option<std::sync::Arc<std::sync::Mutex<Meter>>>,
}
impl ExpectedSeal {
    /// Handoff through an inherited immutable anonymous descriptor. The custody
    /// parent owns provenance of this descriptor; adjacent files/DTOs are never
    /// accepted. No access material is present, and no writer can be recovered.
    pub fn custody_descriptor(&self) -> Result<File> {
        let meter = self.issuance.as_ref().ok_or("pi_read_only_expectation")?;
        let b=serde_json::to_vec(&json!({"binding":self.binding,"identity":self.identity,"ordinal":self.ordinal,"digest":self.digest,"publication":self.publication,"registry":self.registry})).map_err(|_|"pi_custody_json")?;
        let mut meter = meter.lock().map_err(|_| "pi_meter")?;
        let charge = b.len().checked_add(10).ok_or("pi_aggregate")?;
        if meter.poisoned || meter.charge.checked_add(charge).ok_or("pi_aggregate")? > MAX_STORE {
            return Err("pi_aggregate");
        }
        meter.charge += charge;
        let label = CString::new("pi-custody").unwrap();
        let fd = unsafe { libc::memfd_create(label.as_ptr(), libc::MFD_ALLOW_SEALING) };
        if fd < 0 {
            return Err("pi_custody_fd");
        }
        let mut f = unsafe { File::from_raw_fd(fd) };
        if f.write_all(&b).is_err() {
            meter.poisoned = true;
            return Err("pi_custody_write");
        }
        if unsafe {
            libc::fcntl(
                fd,
                libc::F_ADD_SEALS,
                libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL,
            )
        } < 0
        {
            return Err("pi_custody_seal");
        }
        Ok(f)
    }
    /// Must be the descriptor directly inherited from the authenticated custody
    /// parent. Verifying seals proves immutability, not parent identity; the host
    /// owns that production-path claim and may not substitute a caller file.
    pub fn from_custody_descriptor(f: &File) -> Result<Self> {
        use std::os::unix::fs::FileExt;
        let seals = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GET_SEALS) };
        let required =
            libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
        if seals < 0 || seals & required != required {
            return Err("pi_custody_seal");
        }
        let n = io(f.metadata())?.len();
        if n > 8192 {
            return Err("pi_custody_bound");
        }
        let mut b = vec![0; n as usize];
        io(f.read_exact_at(&mut b, 0))?;
        let v = parse_json(&b)?;
        Ok(Self {
            binding: serde_json::from_value(v["binding"].clone())
                .map_err(|_| "pi_custody_binding")?,
            identity: serde_json::from_value(v["identity"].clone())
                .map_err(|_| "pi_custody_root")?,
            ordinal: v["ordinal"].as_u64().ok_or("pi_custody_ordinal")?,
            digest: v["digest"].as_str().ok_or("pi_custody_digest")?.into(),
            publication: v["publication"]
                .as_str()
                .ok_or("pi_custody_publication")?
                .into(),
            registry: serde_json::from_value(v["registry"].clone())
                .map_err(|_| "pi_custody_registry")?,
            issuance: None,
        })
    }
}
/// Read/reconcile has no access acquisition, transport, evolution or write API.
pub struct SealedReader {
    dir: File,
    binding: Binding,
    expected: ExpectedSeal,
}
impl SealedReader {
    pub fn open(root: &Path, expected: ExpectedSeal) -> Result<Self> {
        let dir = open_dir(root)?;
        let m = io(dir.metadata())?;
        if (m.dev(), m.ino()) != expected.identity {
            return Err("pi_external_root");
        }
        Ok(Self {
            dir,
            binding: expected.binding.clone(),
            expected,
        })
    }
    pub fn read(&self, ordinal: u64) -> Result<Snapshot> {
        if let Some(expected) = &self.expected.registry {
            let name = CString::new(expected.directory.as_str()).map_err(|_| "pi_registry_name")?;
            if expected.directory.contains('/')
                || expected.directory == "."
                || expected.directory == ".."
            {
                return Err("pi_registry_name");
            }
            let fd = unsafe {
                libc::openat(
                    self.dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err("pi_registry_read");
            }
            let dir = unsafe { File::from_raw_fd(fd) };
            let md = io(dir.metadata())?;
            if (md.dev(), md.ino()) != expected.identity {
                return Err("pi_registry_root");
            }
            let image = read(&dir, &format!("registry-{:04}.sqlite", expected.generation))?;
            let metadata = read(&dir, &format!("registry-{:04}.json", expected.generation))?;
            if hash(&image) != expected.image || hash(&metadata) != expected.metadata {
                return Err("pi_registry_external_publication");
            }
            let metadata = parse_json(&metadata)?;
            if metadata["profile"] != expected.profile
                || metadata["generation"] != expected.generation
                || metadata["image"] != expected.image
            {
                return Err("pi_registry_final_context");
            }
        }
        if ordinal == 0 || ordinal > 64 || ordinal != self.expected.ordinal {
            return Err("pi_ordinal");
        };
        let mut predecessor: Option<String> = None;
        let mut last: Option<Snapshot> = None;
        for i in 1..=ordinal {
            let bytes = read(&self.dir, &format!("snapshot-{i:04}.json"))?;
            let v: Value = parse_json(&bytes)?;
            if i == ordinal && hash(&bytes) != self.expected.publication {
                return Err("pi_external_publication");
            }
            let p = &v["payload"];
            let b = serde_json::to_vec(p).map_err(|_| "pi_snapshot_json")?;
            let digest = hash(&b);
            if v["sha256"] != digest
                || p["schema"] != PROFILE
                || p["binding"] != serde_json::to_value(&self.binding).unwrap()
                || p["ordinal"] != i
                || p["predecessor"] != serde_json::to_value(&predecessor).unwrap()
            {
                return Err("pi_seal_join");
            };
            let header = p["header"].clone();
            let entries = p["entries"].as_array().ok_or("pi_entries")?.clone();
            let leaf = p["leaf"].as_str().ok_or("pi_leaf")?.to_owned();
            validate(&self.binding, &header, &entries, &leaf)?;
            if let Some(old) = &last {
                if old.header != header
                    || entries.len() <= old.entries.len()
                    || !entries.starts_with(&old.entries)
                {
                    return Err("pi_sealed_prefix");
                }
            }
            predecessor = Some(digest.clone());
            last = Some(Snapshot {
                ordinal: i,
                digest,
                binding: self.binding.clone(),
                header,
                entries,
                leaf,
            });
        }
        let s = last.ok_or("pi_no_snapshot")?;
        if s.digest != self.expected.digest {
            return Err("pi_external_seal");
        }
        Ok(s)
    }
}
