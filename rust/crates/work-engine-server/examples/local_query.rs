use std::{env, time::Duration};
use work_engine_server::LocalClient;
fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        std::process::exit(2);
    }
    let client = LocalClient::new(
        args[1].parse().unwrap(),
        args[2].clone(),
        Duration::from_secs(45),
    )
    .unwrap();
    let arguments = serde_json::from_str(&args[3]).unwrap();
    match client.query("list_sessions", arguments, "client-fixture") {
        Ok(result) => println!("{result}"),
        Err(error) => {
            println!("{}", serde_json::json!({"transport_error":error}));
            std::process::exit(1);
        }
    }
}
