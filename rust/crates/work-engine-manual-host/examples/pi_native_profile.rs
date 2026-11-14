//! Read-only prerequisite diagnostic. Never creates/starts a workload.
use work_engine_manual_host::pi_docker::native_memory::{
    diagnose_attach_backing, disable_process_dumps, observe_backing, observe_docker_service_relay,
    observe_local_docker_socket_peer,
};
fn main() {
    if disable_process_dumps().is_err() {
        println!("{{\"refusal\":\"controller_dump_control\"}}");
        std::process::exit(2);
    }
    // No candidate data is accepted by this executable. Observations below are
    // fixed kernel facts, not SDK/provider/controller output or authority.
    let value = serde_json::json!({
        "schema":"work-engine.pi-native-profile-diagnostic.v1",
        "controller":observe_backing(std::process::id()),
        "docker_socket_peer":observe_local_docker_socket_peer(),
        "docker_service_relay":observe_docker_service_relay(),
        "diagnosis":diagnose_attach_backing(),
        "runtime_qualified":false,
        "containers_created":0,
        "sdk_executed":false
    });
    println!("{}", serde_json::to_string(&value).unwrap());
}
