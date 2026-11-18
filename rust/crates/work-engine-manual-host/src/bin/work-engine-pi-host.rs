//! Pi Host controller mode (`native-realization` `PI_HOST_PROTOCOL.md`):
//! `work-engine-pi-host serve --config <serve-config.json>` (or
//! `dynamic-tools`, which prints the pinned tool set). Stdout carries
//! only the protocol; the run record goes to stderr and, once the job's
//! state directory exists, to `pi-host-serve-record.json` in it. Live routes take the
//! embedder's credentials file (`WORK_ENGINE_HOSTED_CREDENTIALS`, never part
//! of the job); without it only the job's loopback routes can be reached.
use std::io::BufReader;
use work_engine_manual_host::pi_docker::coding::{
    hosted::LoopbackTransport, https::HttpsTransport,
};
use work_engine_manual_host::pi_host_serve::{Out, serve, spawn_reader};

fn main() {
    work_engine_manual_host::install_signals();
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "dynamic-tools" {
        // The pinned `dynamicTools` a controller's `thread/start` must carry.
        println!("{}", native_realization::pi_host::tools::dynamic_tools());
        return;
    }
    if args.len() != 4 || args[1] != "serve" || args[2] != "--config" {
        eprintln!("usage: work-engine-pi-host serve --config <serve-config.json>");
        std::process::exit(2);
    }
    let out = Out::new(Box::new(std::io::stdout()));
    let inbox = spawn_reader(BufReader::new(std::io::stdin()));
    let result = serve(
        std::path::Path::new(&args[3]),
        out,
        inbox,
        |host| match std::env::var_os("WORK_ENGINE_HOSTED_CREDENTIALS") {
            Some(path) => host.bind_hosted_transport(Box::new(HttpsTransport::from_file(
                std::path::Path::new(&path),
            )?)),
            None => host.bind_hosted_transport(Box::new(LoopbackTransport::new()?)),
        },
    );
    match result {
        Ok(record) => eprintln!("{record}"),
        Err(e) => {
            eprintln!(
                "{}",
                serde_json::json!({"status":"refused","error":e.to_string()})
            );
            std::process::exit(1);
        }
    }
}
