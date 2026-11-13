//! Test helper (feature `fixture`): runs one store operation in its own process with an optional
//! fault so tests can SIGKILL or pause it at a publication boundary.
//!
//! `p1f-helper publish <dir> <request.json>` | `p1f-helper revoke <dir> <grant_id>`
//! Env: `P1F_FAULT=<point>:<action>` with point in before_begin|after_begin|before_commit|after_commit
//! and action in kill|pause. `P1F_SYNC=<dir>` is where pause markers (`at-<point>`, `resume-<point>`)
//! live. `P1F_GO=<file>` makes the process wait for that file before it starts.
use std::time::Duration;
use work_engine_authority_store::store::{
    FaultAction, FaultPoint, PublishOutcome, PublishRequest, RevokeOutcome, Store, Write,
};

fn point(s: &str) -> FaultPoint {
    match s {
        "before_begin" => FaultPoint::BeforeBegin,
        "after_begin" => FaultPoint::AfterBegin,
        "before_commit" => FaultPoint::BeforeCommit,
        "after_commit" => FaultPoint::AfterCommit,
        _ => panic!("bad fault point"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Ok(go) = std::env::var("P1F_GO") {
        while !std::path::Path::new(&go).exists() {
            std::thread::sleep(Duration::from_micros(200));
        }
    }
    let dir = std::path::PathBuf::from(&args[2]);
    let busy = Duration::from_secs(30);
    let mut store = match Store::open(&dir, busy) {
        Ok(s) => s,
        Err(e) => {
            println!("{{\"kind\":\"open_error\",\"msg\":{:?}}}", e.to_string());
            std::process::exit(3);
        }
    };
    if let Ok(f) = std::env::var("P1F_FAULT") {
        let (p, action) = f.split_once(':').expect("point:action");
        let (p, action) = (point(p), action.to_owned());
        let sync = std::env::var("P1F_SYNC").unwrap_or_default();
        store.set_hook(Box::new(move |at, _| {
            if at == p {
                match action.as_str() {
                    "kill" => unsafe {
                        libc::kill(libc::getpid(), libc::SIGKILL);
                    },
                    "pause" => {
                        let name = format!("{at:?}");
                        std::fs::write(
                            std::path::Path::new(&sync).join(format!("at-{name}")),
                            b"x",
                        )
                        .unwrap();
                        while !std::path::Path::new(&sync)
                            .join(format!("resume-{name}"))
                            .exists()
                        {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                    }
                    _ => {}
                }
            }
            FaultAction::Continue
        }));
    }
    let line = match args[1].as_str() {
        "publish" => {
            let req: PublishRequest =
                serde_json::from_slice(&std::fs::read(&args[3]).unwrap()).unwrap();
            match store.fixture_publish(&req) {
                Write::Done(PublishOutcome::Published {
                    result_id,
                    seq,
                    replayed,
                }) => format!(
                    "{{\"kind\":\"published\",\"result_id\":{result_id:?},\"seq\":{seq},\"replayed\":{replayed}}}"
                ),
                Write::Done(PublishOutcome::Refused {
                    reason,
                    seq,
                    replayed,
                }) => format!(
                    "{{\"kind\":\"refused\",\"reason\":{reason:?},\"seq\":{seq},\"replayed\":{replayed}}}"
                ),
                Write::Done(PublishOutcome::Conflict) => "{\"kind\":\"conflict\"}".into(),
                Write::NotCommitted(m) => format!("{{\"kind\":\"not_committed\",\"msg\":{m:?}}}"),
                Write::Unknown(m) => format!("{{\"kind\":\"unknown\",\"msg\":{m:?}}}"),
            }
        }
        "revoke" => match store.revoke_grant(&args[3]) {
            Write::Done(RevokeOutcome::Effective { seq }) => {
                format!("{{\"kind\":\"revoked\",\"seq\":{seq}}}")
            }
            Write::Done(o) => format!(
                "{{\"kind\":\"revoke_other\",\"detail\":{:?}}}",
                format!("{o:?}")
            ),
            Write::NotCommitted(m) => format!("{{\"kind\":\"not_committed\",\"msg\":{m:?}}}"),
            Write::Unknown(m) => format!("{{\"kind\":\"unknown\",\"msg\":{m:?}}}"),
        },
        _ => panic!("bad subcommand"),
    };
    println!("{line}");
}
