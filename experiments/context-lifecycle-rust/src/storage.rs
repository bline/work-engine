use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{File, OpenOptions};
use std::path::Path;

/// The lifetime lock is held until after the database connection is dropped.
pub(crate) struct Store {
    connection: Connection,
    _lock: File,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let path = if path.exists() {
            path.canonicalize()?
        } else {
            path.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()?
                .join(
                    path.file_name()
                        .ok_or_else(|| anyhow::anyhow!("database file name required"))?,
                )
        };
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path.with_extension("lock"))?;
        lock.try_lock()
            .map_err(|e| anyhow::anyhow!("exclusive coordinator lock unavailable: {e}"))?;
        let connection = Connection::open(&path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS state (singleton INTEGER PRIMARY KEY CHECK(singleton=1), body TEXT NOT NULL);")?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }
    pub fn load<T: DeserializeOwned + Default>(&self) -> Result<T> {
        let mut statement = self
            .connection
            .prepare("SELECT body FROM state WHERE singleton=1")?;
        let mut rows = statement.query([])?;
        match rows.next()? {
            Some(row) => Ok(serde_json::from_str(&row.get::<_, String>(0)?)?),
            None => Ok(T::default()),
        }
    }
    pub fn save(&mut self, value: &impl Serialize) -> Result<()> {
        let body = serde_json::to_string(value)?;
        let transaction = self.connection.transaction()?;
        transaction.execute("INSERT INTO state(singleton,body) VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET body=excluded.body", params![body])?;
        transaction.commit()?;
        Ok(())
    }
}
