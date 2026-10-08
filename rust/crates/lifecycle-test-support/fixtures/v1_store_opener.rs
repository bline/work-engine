//! Copied into an immutable base workspace and compiled there by the S4 contract test.
use std::{env,fs,path::PathBuf,process};

use lifecycle_store::SqliteLifecycleStore;
use rusqlite::{Connection,OpenFlags};

fn main() {
    let root=PathBuf::from(env::args_os().nth(1).expect("store root"));
    let database=root.join("lifecycle.sqlite");
    let before=fs::read(&database).expect("read future database before old opener");
    let inspection=Connection::open_with_flags(&database,OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read future schema");
    let version:i64=inspection.query_row("PRAGMA user_version",[],|row|row.get(0)).unwrap();
    assert_eq!(version,2,"test must present a future schema to the base v1 opener");
    drop(inspection);
    if SqliteLifecycleStore::open(&root,"issuer-contract".into()).is_ok() {
        eprintln!("base v1 opener accepted future v2 schema");process::exit(2);
    }
    let after=fs::read(&database).expect("read database after old refusal");
    if before!=after {eprintln!("old opener mutated future database");process::exit(3);}
    println!("base v1 opener refused v2 before mutation");
}
