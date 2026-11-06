mod admission;
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
    let (initialize, input) = admission::StartupInput::parse(&args)?;
    if initialize {
        let startup = input.load(true).map_err(|e| e.to_string())?;
        let digest = startup.digest().to_owned();
        let snapshot = review_workflow::Workflow::initialize(startup).map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::json!({"version":1,"status":"initialized","startupDigest":digest,"campaign":snapshot})
        );
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(control::serve(input))
}
