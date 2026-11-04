//! Local command client: JSON request is read from stdin, never argv.
use std::{io::Read, time::Duration};
use work_engine_server::LocalClient;
fn main() {
    let mut args = std::env::args().skip(1);
    let endpoint = args
        .next()
        .expect("endpoint")
        .parse()
        .expect("socket address");
    let credential = args.next().expect("credential file");
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("stdin");
    let request: serde_json::Value = serde_json::from_str(&input).expect("JSON request");
    let client =
        LocalClient::new(endpoint, credential, Duration::from_secs(45)).expect("client config");
    let result = client.command(
        request["operation"].as_str().expect("operation"),
        request["arguments"].clone(),
        "local-command",
    );
    match result {
        Ok(value) => println!("{value}"),
        Err(code) => {
            println!("{}", serde_json::json!({"transport_error":code}));
            std::process::exit(1);
        }
    }
}
