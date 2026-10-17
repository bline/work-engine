use std::{env, fs, path::PathBuf, process};

use lifecycle_service::{ServiceConfig, run};

#[tokio::main]
async fn main() {
    let mut args = env::args_os().skip(1);
    let Some(first) = args.next() else {
        eprintln!(
            "usage: lifecycle-service <config.json> | --development-selected <selection.json>"
        );
        process::exit(2);
    };
    let result = if first == "--development-selected" {
        match (args.next(), args.next()) {
            (Some(path), None) => {
                lifecycle_service::development_startup::run_selected(&PathBuf::from(path))
                    .await
                    .map_err(|error| error.to_string())
            }
            _ => Err("usage: lifecycle-service --development-selected <selection.json>".into()),
        }
    } else if args.next().is_some() {
        Err("usage: lifecycle-service <config.json>".into())
    } else {
        let config = fs::read(first)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<ServiceConfig>(&bytes).map_err(|error| error.to_string())
            });
        match config {
            Ok(config) => run(config).await.map_err(|error| error.to_string()),
            Err(error) => Err(error),
        }
    };
    if let Err(error) = result {
        eprintln!("lifecycle-service: {error}");
        process::exit(1);
    }
}
