//! Process-level tests (fixture-scoped): termination at each publication boundary (T1) and the
//! revoke-versus-publish ordering (T5). Concurrency evidence only; it says nothing about power
//! loss. The helper binary runs one store operation per process.
#![cfg(feature = "fixture")]
mod common;
use common::harness::*;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use work_engine_authority_store::store::{self, PublishRequest, Reconciled};

const HELPER: &str = env!("CARGO_BIN_EXE_p1f-helper");

struct Seed {
    h: H,
    req: PublishRequest,
    req_path: PathBuf,
}
fn seed() -> Seed {
    let h = harness();
    h.store
        .connection()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    let req = h.request();
    let req_path = h.env.root.join("request.json");
    std::fs::write(&req_path, serde_json::to_vec(&req).unwrap()).unwrap();
    Seed { h, req, req_path }
}
fn spawn(args: &[&str], envs: &[(&str, &str)]) -> Child {
    let mut c = Command::new(HELPER);
    c.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    for (k, v) in envs {
        c.env(k, v);
    }
    c.spawn().unwrap()
}
fn finish(c: Child) -> (Option<i32>, Option<i32>, String) {
    use std::os::unix::process::ExitStatusExt;
    let o = c.wait_with_output().unwrap();
    (
        o.status.code(),
        o.status.signal(),
        String::from_utf8_lossy(&o.stdout).trim().to_owned(),
    )
}
fn wait_for(p: &Path) {
    let t = Instant::now();
    while !p.exists() {
        assert!(
            t.elapsed() < Duration::from_secs(20),
            "timeout waiting for {p:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("helper output {s:?}: {e}"))
}

#[test]
fn t1_kill_at_each_publication_boundary_then_reconcile() {
    let mut summary = Vec::new();
    for (point, expect_committed) in [
        ("before_begin", false),
        ("after_begin", false),
        ("before_commit", false),
        ("after_commit", true),
    ] {
        let s = seed();
        let dir = s.h.dir.to_str().unwrap().to_owned();
        let c = spawn(
            &["publish", &dir, s.req_path.to_str().unwrap()],
            &[("P1F_FAULT", &format!("{point}:kill"))],
        );
        let (code, sig, out) = finish(c);
        assert_eq!(
            sig,
            Some(9),
            "{point}: killed by SIGKILL (code {code:?}, out {out:?})"
        );
        let rec = store::reconcile(&s.h.dir, &s.req.operation_id).unwrap();
        match (&rec, expect_committed) {
            (Reconciled::Absent, false) | (Reconciled::Committed { .. }, true) => {}
            other => panic!("{point}: {other:?}"),
        }
        assert_eq!(store::integrity_check(&s.h.dir).unwrap(), "ok");
        let results: i64 = store::open_read_only(&s.h.dir)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM results", [], |r| r.get(0))
            .unwrap();
        let receipts: i64 = store::open_read_only(&s.h.dir)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            (results, receipts),
            if expect_committed { (1, 1) } else { (0, 0) },
            "{point}: no partial rows"
        );
        // Recovery is a new owner-driven operation: replaying the same request on a fresh process.
        let again = finish(spawn(&["publish", &dir, s.req_path.to_str().unwrap()], &[]));
        let j = json(&again.2);
        assert_eq!(j["kind"], "published", "{point}");
        assert_eq!(j["replayed"], expect_committed, "{point}");
        summary.push(format!(
            "{point}: killed(9) reconcile={} then retry replayed={}",
            if expect_committed {
                "Committed"
            } else {
                "Absent"
            },
            expect_committed
        ));
    }
    println!("T1 {summary:?}");
}

#[test]
fn t5_paused_publisher_blocks_the_revoker_so_the_publication_orders_first() {
    let s = seed();
    let dir = s.h.dir.to_str().unwrap().to_owned();
    let sync = s.h.env.root.join("sync");
    std::fs::create_dir(&sync).unwrap();
    let sy = sync.to_str().unwrap();
    let publisher = spawn(
        &["publish", &dir, s.req_path.to_str().unwrap()],
        &[("P1F_FAULT", "before_commit:pause"), ("P1F_SYNC", sy)],
    );
    wait_for(&sync.join("at-BeforeCommit"));
    let mut revoker = spawn(&["revoke", &dir, s.h.grant.grant_id()], &[]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        revoker.try_wait().unwrap().is_none(),
        "revoke must wait for the publisher's write lock"
    );
    std::fs::write(sync.join("resume-BeforeCommit"), b"go").unwrap();
    let p = json(&finish(publisher).2);
    let r = json(&finish(revoker).2);
    assert_eq!(p["kind"], "published");
    assert_eq!(r["kind"], "revoked");
    assert!(
        p["seq"].as_i64() < r["seq"].as_i64(),
        "publication committed before the revocation"
    );
}

#[test]
fn t5_paused_revoker_blocks_the_publisher_so_the_publish_refuses() {
    let s = seed();
    let dir = s.h.dir.to_str().unwrap().to_owned();
    let sync = s.h.env.root.join("sync");
    std::fs::create_dir(&sync).unwrap();
    let sy = sync.to_str().unwrap();
    let revoker = spawn(
        &["revoke", &dir, s.h.grant.grant_id()],
        &[("P1F_FAULT", "before_commit:pause"), ("P1F_SYNC", sy)],
    );
    wait_for(&sync.join("at-BeforeCommit"));
    let mut publisher = spawn(&["publish", &dir, s.req_path.to_str().unwrap()], &[]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        publisher.try_wait().unwrap().is_none(),
        "publish must wait for the revoker's write lock"
    );
    std::fs::write(sync.join("resume-BeforeCommit"), b"go").unwrap();
    let r = json(&finish(revoker).2);
    let p = json(&finish(publisher).2);
    assert_eq!(r["kind"], "revoked");
    assert_eq!(p["kind"], "refused");
    assert_eq!(p["reason"], "GrantRevoked");
    assert!(p["seq"].as_i64() > r["seq"].as_i64());
}

#[test]
fn t5_process_pairs_n200_publication_never_commits_after_revocation() {
    let template = seed();
    let dbfile = template.h.dir.join(store::DB_FILE);
    let n: usize = std::env::var("P1F_T5_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let scratch = template.h.env.root.join("pairs");
    std::fs::create_dir(&scratch).unwrap();
    let (mut published_first, mut revoked_first) = (0usize, 0usize);
    for i in 0..n {
        let dir = scratch.join(format!("run{i}"));
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        }
        std::fs::copy(&dbfile, dir.join(store::DB_FILE)).unwrap();
        let go = dir.join("go");
        let d = dir.to_str().unwrap();
        let envs = [("P1F_GO", go.to_str().unwrap())];
        let publisher = spawn(&["publish", d, template.req_path.to_str().unwrap()], &envs);
        let revoker = spawn(&["revoke", d, template.h.grant.grant_id()], &envs);
        std::fs::write(&go, b"go").unwrap();
        let p = json(&finish(publisher).2);
        let r = json(&finish(revoker).2);
        assert_eq!(r["kind"], "revoked", "run {i}");
        let rseq = r["seq"].as_i64().unwrap();
        match p["kind"].as_str().unwrap() {
            "published" => {
                assert!(
                    p["seq"].as_i64().unwrap() < rseq,
                    "run {i}: publish committed after revoke: {p} {r}"
                );
                published_first += 1;
            }
            "refused" => {
                assert_eq!(p["reason"], "GrantRevoked", "run {i}");
                assert!(p["seq"].as_i64().unwrap() > rseq, "run {i}");
                revoked_first += 1;
            }
            other => panic!("run {i}: {other} {p}"),
        }
        let c = store::open_read_only(&dir).unwrap();
        let bad: i64 = c.query_row("SELECT COUNT(*) FROM results r JOIN grants g ON g.grant_id=r.grant_id WHERE g.revoked_seq IS NOT NULL AND r.seq > g.revoked_seq", [], |r| r.get(0)).unwrap();
        assert_eq!(bad, 0, "run {i}");
        let dup: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM (SELECT seq FROM log GROUP BY seq HAVING COUNT(*)>1)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dup, 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
    println!(
        "T5 n={n} publish_ordered_first={published_first} revoke_ordered_first={revoked_first}"
    );
    assert_eq!(published_first + revoked_first, n);
}
