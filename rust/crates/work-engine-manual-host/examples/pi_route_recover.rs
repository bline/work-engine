//! Fresh-process routing recovery from a native store's durable route controls.
//! Read-only: no access material, transport, registry mutation or dispatch.
//! Usage: pi_route_recover <store-dir>
use serde_json::{Value, json};
use std::{fs, path::Path};
use work_engine_manual_host::pi_route::{Attempt, Decision, Policy, Router};

fn events(dir: &Path, prefix: &str) -> Result<Vec<(String, Value)>, &'static str> {
    let mut out = vec![];
    for e in fs::read_dir(dir).map_err(|_| "store_read")? {
        let name = e
            .map_err(|_| "store_read")?
            .file_name()
            .into_string()
            .map_err(|_| "store_name")?;
        if name.starts_with(prefix) && name.ends_with(".json") {
            let v: Value =
                serde_json::from_slice(&fs::read(dir.join(&name)).map_err(|_| "store_read")?)
                    .map_err(|_| "control_json")?;
            out.push((name, v));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
fn run() -> Result<Value, &'static str> {
    let dir = std::env::args().nth(1).ok_or("store_required")?;
    let dir = Path::new(&dir);
    let base = events(dir, "control-route-000")?
        .pop()
        .ok_or("policy_missing")?
        .1;
    let binding = base["binding"].clone();
    let policy: Policy =
        serde_json::from_value(base["event"]["policy"].clone()).map_err(|_| "policy_json")?;
    if base["event"]["policyDigest"] != policy.digest() {
        return Err("policy_digest");
    }
    let mut intents = vec![];
    for (name, v) in events(dir, "control-route-")? {
        if v["binding"] != binding {
            return Err("foreign_binding");
        }
        if name != "control-route-000.json" && v["event"]["decision"] == "attempt" {
            intents.push(
                serde_json::from_value::<Attempt>(v["event"]["intent"].clone())
                    .map_err(|_| "intent_json")?,
            );
        }
    }
    let mut settles = vec![];
    for (_, v) in events(dir, "control-settle-")? {
        if v["binding"] != binding {
            return Err("foreign_binding");
        }
        settles.push(
            serde_json::from_value::<Attempt>(v["event"]["attempt"].clone())
                .map_err(|_| "settle_json")?,
        );
    }
    let mut router = Router::recover(policy, None, &intents, &settles)?;
    let recovered = router.evidence();
    // What this router would now permit for each request it has seen.
    let mut requests: Vec<u64> = router.attempts().iter().map(|a| a.request).collect();
    requests.dedup();
    let model = router.policy().model.clone();
    let after: Vec<Value> = requests
        .iter()
        .map(|&q| match router.next(q, &model, 0) {
            Decision::Attempt { route, attempt } => {
                json!({"request":q,"decision":"attempt","route":route,"attempt":attempt})
            }
            Decision::Unavailable(r) => json!({"request":q,"decision":"unavailable","reason":r}),
        })
        .collect();
    Ok(
        json!({"classification":"fresh-process-route-recovery","intents":intents.len(),"settlements":settles.len(),"router":recovered,"nextForSeenRequests":after}),
    )
}
fn main() {
    match run() {
        Ok(v) => println!("{v}"),
        Err(code) => {
            eprintln!("pi_route_recover: {code}");
            std::process::exit(1)
        }
    }
}
