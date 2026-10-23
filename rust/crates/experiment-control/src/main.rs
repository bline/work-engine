use base64::{Engine, engine::general_purpose::STANDARD};
use experiment_control::{
    HARD_MAX_BYTES, Input, MAX_ARTIFACT_BYTES, MAX_BYTES, apply, digest, strict,
};
use std::{
    fs::File,
    io::{self, Read},
};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--help" => {
            println!(
                "experiment-control: pure reducer, JSON Input on stdin -> Output on stdout. Use experiment_control.py for durable operations.\n--schema emits the Input schema.\nsnapshot FILE emits bounded inline bytes + SHA-256; it records no exposure."
            );
            Ok(())
        }
        [flag] if flag == "--schema" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schemars::schema_for!(Input))
                    .map_err(|_| "schema encoding failed")?
            );
            Ok(())
        }
        [flag, path] if flag == "snapshot" => {
            let metadata =
                std::fs::symlink_metadata(path).map_err(|_| "snapshot source unavailable")?;
            if !metadata.is_file() {
                return Err("snapshot source must be a regular non-symlink file".into());
            }
            let file = File::open(path).map_err(|_| "snapshot source unavailable")?;
            if !file
                .metadata()
                .map_err(|_| "snapshot metadata unavailable")?
                .is_file()
            {
                return Err("snapshot source must be regular".into());
            }
            let mut bytes = vec![];
            file.take(MAX_ARTIFACT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "snapshot read failed")?;
            if bytes.len() > MAX_ARTIFACT_BYTES {
                return Err("snapshot exceeds 512 KiB".into());
            }
            println!(
                "{}",
                serde_json::json!({"sha256":digest(&bytes),"data_base64":STANDARD.encode(bytes)})
            );
            Ok(())
        }
        [] => {
            let mut bytes = vec![];
            io::stdin()
                .take((HARD_MAX_BYTES + MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| "input read failed")?;
            if bytes.len() > HARD_MAX_BYTES + MAX_BYTES {
                return Err("input exceeds size bound".into());
            }
            let output = apply(strict::parse(&bytes)?)?;
            println!(
                "{}",
                serde_json::to_string(&output).map_err(|_| "output encoding failed")?
            );
            Ok(())
        }
        _ => Err("unsupported arguments; use --help".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
