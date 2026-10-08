use std::{env, fs, process};

use lifecycle_service::{ServiceConfig, run};

#[tokio::main]
async fn main() {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: lifecycle-service <config.json>");
        process::exit(2);
    };
    let result = fs::read(path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<ServiceConfig>(&bytes).map_err(|error| error.to_string())
        });
    let result = match result {
        Ok(config) => run(config).await.map_err(|error| error.to_string()),
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        eprintln!("lifecycle-service: {error}");
        process::exit(1);
    }
}
