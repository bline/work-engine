mod control;
mod worker;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 || args[1] != "--startup" {
        return Err(
            "usage: work-engine-review-host initialize|serve --startup <absolute-file>".into(),
        );
    }
    let path = std::path::PathBuf::from(&args[2]);
    if !path.is_absolute() {
        return Err("startup path must be absolute".into());
    }
    if args[0] == "initialize" {
        let startup =
            review_workflow::startup::TrustedStartup::load(&path).map_err(|e| e.to_string())?;
        let digest = startup.digest().to_owned();
        let snapshot = review_workflow::Workflow::initialize(startup).map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::json!({"version":1,"status":"initialized","startupDigest":digest,"campaign":snapshot})
        );
        return Ok(());
    }
    if args[0] != "serve" {
        return Err("unsupported host mode".into());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(control::serve(path))
}
