//! Dedicated authoritative native registry. No live database import, generic
//! SQLite path, writer reopen, or fork of existing reservations is supported.
use super::*;
use std::sync::{Arc, Mutex};
struct State {
    sink: LiveStore,
    image: Vec<u8>,
    generation: u64,
    active: bool,
    profile: String,
    parent: (std::path::PathBuf, (u64, u64)),
}
#[derive(Clone)]
pub struct CheckedRegistry {
    state: Arc<Mutex<State>>,
}
pub struct RegistryConnection {
    db: rusqlite::Connection,
    state: Arc<Mutex<State>>,
    generation: std::cell::Cell<u64>,
}
impl std::ops::Deref for RegistryConnection {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}
impl std::ops::DerefMut for RegistryConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}
impl LiveStore {
    /// The directory is itself the configured database path. Generic SQLite
    /// consumers fail opening a directory; only this issued backend can write.
    pub fn create_registry(&self, path: &Path) -> Result<CheckedRegistry> {
        self.current()?;
        if path.parent() != Some(self.root.as_path()) || path.file_name().is_none() {
            return Err("pi_registry_namespace");
        }
        self.check(&json!({"database":path.to_str().ok_or("pi_registry_path")?}))?;
        let mut meter = self.meter.lock().map_err(|_| "pi_meter")?;
        if meter.registry_created {
            return Err("pi_registry_exists");
        }
        // Failure after this point cannot permit a second authority namespace.
        meter.registry_created = true;
        // Charge the full destination before mkdir; partial creation is never refunded.
        let charge = path.as_os_str().len();
        if meter.charge.checked_add(charge).ok_or("pi_aggregate")? > MAX_STORE - 3 * MAX_SNAPSHOT {
            return Err("pi_aggregate");
        }
        meter.charge += charge;
        let dir = match checked_directory(path, &self.original, Some(&self.dir)) {
            Ok(dir) => dir,
            Err(e) => {
                meter.poisoned = true;
                return Err(e);
            }
        };
        let md = io(dir.metadata())?;
        let sink = LiveStore {
            dir,
            original: self.original.clone(),
            binding: self.binding.clone(),
            expires: self.expires,
            next: 1,
            head: None,
            last: None,
            meter: self.meter.clone(),
            pending: None,
            root: path.to_owned(),
            identity: (md.dev(), md.ino()),
        };
        Ok(CheckedRegistry {
            state: Arc::new(Mutex::new(State {
                sink,
                image: vec![],
                generation: 0,
                active: false,
                profile: String::new(),
                parent: (self.root.clone(), self.identity),
            })),
        })
    }
}
impl CheckedRegistry {
    /// Bind exactly the descriptor's protected namespace and profile once.
    pub fn bind(&self, path: &Path, profile: &str) -> Result<()> {
        let mut s = self.state.lock().map_err(|_| "pi_registry_lock")?;
        s.sink.current()?;
        let parent = io(fs::symlink_metadata(&s.parent.0))?;
        if !parent.is_dir()
            || parent.file_type().is_symlink()
            || (parent.dev(), parent.ino()) != s.parent.1
        {
            return Err("pi_registry_parent");
        }
        if path != s.sink.root || !s.profile.is_empty() {
            return Err("pi_registry_binding");
        }
        s.sink.check(&json!({"profile":profile}))?;
        s.profile = profile.into();
        Ok(())
    }
    pub fn checkout(&self) -> Result<RegistryConnection> {
        let mut s = self.state.lock().map_err(|_| "pi_registry_lock")?;
        s.sink.current()?;
        let parent = io(fs::symlink_metadata(&s.parent.0))?;
        if !parent.is_dir()
            || parent.file_type().is_symlink()
            || (parent.dev(), parent.ino()) != s.parent.1
        {
            return Err("pi_registry_parent");
        }
        if s.active || s.profile.is_empty() {
            return Err("pi_registry_ordering");
        }
        let mut db = rusqlite::Connection::open_in_memory().map_err(|_| "pi_registry_memory")?;
        if !s.image.is_empty() {
            db.deserialize_read_exact(rusqlite::MAIN_DB, s.image.as_slice(), s.image.len(), false)
                .map_err(|_| "pi_registry_memory")?;
        }
        db.execute_batch(
            "PRAGMA temp_store=MEMORY; PRAGMA journal_mode=MEMORY; PRAGMA foreign_keys=ON;",
        )
        .map_err(|_| "pi_registry_memory")?;
        s.active = true;
        Ok(RegistryConnection {
            db,
            state: self.state.clone(),
            generation: std::cell::Cell::new(s.generation),
        })
    }
}
impl RegistryConnection {
    pub(super) fn meter(&self) -> Result<Arc<Mutex<Meter>>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "pi_registry_lock")?
            .sink
            .meter
            .clone())
    }
    pub fn check_current(&self) -> Result<()> {
        let s = self.state.lock().map_err(|_| "pi_registry_lock")?;
        s.sink.current()?;
        let parent = io(fs::symlink_metadata(&s.parent.0))?;
        if !parent.is_dir()
            || parent.file_type().is_symlink()
            || (parent.dev(), parent.ino()) != s.parent.1
        {
            return Err("pi_registry_parent");
        }
        if !s.active || s.generation != self.generation.get() {
            return Err("pi_registry_stale");
        }
        Ok(())
    }
    /// Own the complete final SQLite image, structured text cells, generated
    /// names and envelopes. No caller can obtain a public check permit and then
    /// serialize or retain a different database object.
    pub fn publish(&self, closure: bool) -> Result<()> {
        self.check_current()?;
        if !self.db.is_autocommit() {
            return Err("pi_registry_uncommitted");
        }
        let image = self
            .db
            .serialize(rusqlite::MAIN_DB)
            .map_err(|_| "pi_registry_serialize")?
            .to_vec();
        if image.len() > MAX_SNAPSHOT {
            return Err("pi_registry_bound");
        }
        let mut s = self.state.lock().map_err(|_| "pi_registry_lock")?;
        if image == s.image {
            return Ok(());
        }
        let mut inspect = Inspection {
            a: &s.sink.original,
            bytes: 0,
            records: 0,
            work: 0,
        };
        check_bytes(&image, &s.sink.original)?;
        let mut stmt = self
            .db
            .prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
            .map_err(|_| "pi_registry_schema")?;
        let schema = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| "pi_registry_schema")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| "pi_registry_schema")?;
        if schema.len() > 64 {
            return Err("pi_registry_schema");
        }
        for (table, sql) in schema {
            inspect.text(&table, 0)?;
            inspect.text(&sql, 0)?;
            // sqlite_schema includes table DDL only in this fixed owner schema.
            // Refuse views/triggers/indices rather than follow unbounded SQL.
            if !sql.starts_with("CREATE TABLE ") {
                return Err("pi_registry_schema");
            }
            let query = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
            let mut stmt = self.db.prepare(&query).map_err(|_| "pi_registry_rows")?;
            let columns = stmt.column_count();
            let mut rows = stmt.query([]).map_err(|_| "pi_registry_rows")?;
            let mut count = 0;
            while let Some(row) = rows.next().map_err(|_| "pi_registry_rows")? {
                count += 1;
                if count > 256 {
                    return Err("pi_registry_rows");
                }
                for i in 0..columns {
                    match row.get_ref(i).map_err(|_| "pi_registry_value")? {
                        rusqlite::types::ValueRef::Text(b) => inspect
                            .text(std::str::from_utf8(b).map_err(|_| "pi_registry_utf8")?, 0)?,
                        rusqlite::types::ValueRef::Blob(_) => return Err("pi_registry_blob"),
                        _ => (),
                    }
                }
            }
        }
        let next = s
            .generation
            .checked_add(1)
            .ok_or("pi_registry_generation")?;
        let name = format!("registry-{next:04}.sqlite");
        let digest = hash(&image);
        let predecessor = if s.generation == 0 {
            None
        } else {
            Some(hash(&s.image))
        };
        let envelope=s.sink.check(&json!({"schema":"pi-checked-registry/v1","profile":s.profile,"generation":next,"predecessor":predecessor,"image":digest,"bytes":image.len(),"name":name}))?;
        Inspection {
            a: &s.sink.original,
            bytes: 0,
            records: 0,
            work: 0,
        }
        .text(&name, 0)?;
        let metadata_name = format!("registry-{next:04}.json");
        Inspection {
            a: &s.sink.original,
            bytes: 0,
            records: 0,
            work: 0,
        }
        .text(&metadata_name, 0)?;
        let charge = image.len() + envelope.len() + name.len() + metadata_name.len();
        {
            let mut meter = s.sink.meter.lock().map_err(|_| "pi_meter")?;
            let limit = if closure {
                MAX_STORE
            } else {
                MAX_STORE - 3 * MAX_SNAPSHOT
            };
            if meter.poisoned || meter.charge.checked_add(charge).ok_or("pi_aggregate")? > limit {
                return Err("pi_aggregate");
            }
            meter.charge += charge;
        }
        s.sink.current()?;
        let parent = io(fs::symlink_metadata(&s.parent.0))?;
        if !parent.is_dir()
            || parent.file_type().is_symlink()
            || (parent.dev(), parent.ino()) != s.parent.1
        {
            return Err("pi_registry_parent");
        }
        let seal = RegistrySeal {
            directory: s
                .sink
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("pi_registry_name")?
                .into(),
            identity: s.sink.identity,
            generation: next,
            profile: s.profile.clone(),
            image: digest,
            metadata: hash(&envelope),
        };
        // The exact complete expectation is checked before custody can retain it.
        s.sink
            .check(&serde_json::to_value(&seal).map_err(|_| "pi_registry_seal")?)?;
        if create(&s.sink.dir, &name, &image)
            .and_then(|_| create(&s.sink.dir, &metadata_name, &envelope))
            .is_err()
        {
            s.sink.meter.lock().map_err(|_| "pi_meter")?.poisoned = true;
            return Err("pi_registry_unresolved");
        }
        s.sink.meter.lock().map_err(|_| "pi_meter")?.registry = Some(seal);
        s.image = image;
        s.generation = next;
        self.generation.set(next);
        Ok(())
    }
}
impl Drop for RegistryConnection {
    fn drop(&mut self) {
        if let Ok(mut s) = self.state.lock() {
            s.active = false;
        }
    }
}
