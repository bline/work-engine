//! Fixed MCP transport composition. Task admission owns authority and durable
//! results; Host owns native effects. No transport-local admission journal.
use serde::Deserialize;
use serde_json::{Value, json};
use session_task_admission::{
    canonical_source::Pin,
    pi_pilot::{self, PiOwner},
};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};
use work_engine_manual_host::{
    Artifact, digest,
    pi_docker::{Host, Job},
};
const PROTOCOL: &str = "work-engine.mcp-pi-coding-fixture.v1";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    descriptor: Pin,
    job: Artifact,
    principal: String,
    subject: String,
    handle: String,
    revision: String,
    idempotency_key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    method: String,
    principal: String,
    handle: String,
    revision: String,
    idempotency_key: String,
}
fn call(c: Config, r: Request) -> Result<Value, String> {
    if c.schema != PROTOCOL
        || r.schema != PROTOCOL
        || !["submit", "status", "result"].contains(&r.method.as_str())
    {
        return Err("pi_protocol".into());
    }
    let exe = digest(
        &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    let d = pi_pilot::check_transport(
        &c.descriptor,
        &r.principal,
        &r.handle,
        &r.revision,
        &r.idempotency_key,
        &exe,
    )
    .map_err(str::to_owned)?;
    let t = d.transport;
    if (
        c.principal,
        c.subject,
        c.handle,
        c.revision,
        c.idempotency_key,
    ) != (
        t.principal,
        t.subject,
        t.handle,
        t.revision,
        t.idempotency_key,
    ) {
        return Err("pi_transport_conflict".into());
    }
    let existing = pi_pilot::read(&c.descriptor).map_err(str::to_owned)?;
    if r.method != "submit" || existing["state"] != "not_admitted" {
        return Ok(existing);
    }
    let job: Job = serde_json::from_slice(&c.job.verify().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let owner = PiOwner::open(c.descriptor.clone()).map_err(str::to_owned)?;
    let mut host = Host::admit_signed_fixture(job, owner).map_err(|e| e.to_string())?;
    let _native = host.run().map_err(|e| e.to_string())?;
    pi_pilot::read(&c.descriptor).map_err(str::to_owned)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = (|| -> Result<Value, String> {
        if args.len() != 4
            || args[3] != "rpc"
            || !["--config", "--config-fd"].contains(&args[1].as_str())
        {
            return Err("pi_protocol".into());
        }
        let path = if args[1] == "--config-fd" {
            let fd = args[2].parse::<u32>().map_err(|_| "pi_config")?;
            PathBuf::from(format!("/proc/self/fd/{fd}"))
        } else {
            PathBuf::from(&args[2])
        };
        let bytes = fs::read(&path).map_err(|_| "pi_config")?;
        if bytes.len() > 16384 {
            return Err("pi_config".into());
        }
        let c: Config = serde_json::from_slice(&bytes).map_err(|_| "pi_config")?;
        let mut request = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut request)
            .map_err(|_| "pi_request")?;
        if request.len() > 16384 {
            return Err("pi_request".into());
        }
        let r = serde_json::from_slice(&request).map_err(|_| "pi_request")?;
        call(c, r)
    })();
    let out = match result {
        Ok(v) => {
            json!({"schema_version":1,"protocol":PROTOCOL,"ok":true,"result":{"schema_version":1,"pi_coding":v}})
        }
        Err(e) => json!({"schema_version":1,"protocol":PROTOCOL,"ok":false,"error":e}),
    };
    let _ = writeln!(std::io::stdout(), "{out}");
}
