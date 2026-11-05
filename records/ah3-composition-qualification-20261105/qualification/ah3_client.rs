//! Qualification-only public LocalClient caller; no domain implementation.
use serde_json::{Value, json};
use std::{env, time::Duration};
fn main() {
    let a: Vec<String> = env::args().collect();
    let client = work_engine_server::LocalClient::new(
        a[1].parse().unwrap(), a[2].clone(), Duration::from_secs(30),
    ).unwrap();
    let args: Value = serde_json::from_str(&a[5]).unwrap();
    let result = match a[3].as_str() {
        "query" => client.query(&a[4], args, "ah3-public-client"),
        "task" => client.task(&a[4], args, "ah3-public-client"),
        "command" => client.command(&a[4], args, "ah3-public-client"),
        _ => panic!("unsupported qualification family"),
    };
    match result {
        Ok(value) => println!("{value}"),
        Err(code) => { println!("{}", json!({"transport_error":code})); std::process::exit(1); }
    }
}
