mod admission;
mod completion;
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
    #[cfg(feature = "test-support")]
    if args
        .first()
        .is_some_and(|arg| arg == "completion-fixture-readback")
    {
        let mut input_args = args.clone();
        input_args[0] = "serve".into();
        let (_, input) = admission::StartupInput::parse(&input_args)?;
        let startup = input.load(false).map_err(|e| e.to_string())?;
        let workflow = review_workflow::Workflow::open(startup).map_err(|e| e.to_string())?;
        println!(
            "{}",
            workflow
                .inspect_completion_fixture()
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|arg| arg == "completion-bootstrap" || arg == "completion-serve")
    {
        return completion::run(&args, args[0] == "completion-bootstrap");
    }
    if args.first().is_some_and(|arg| arg == "prepare") {
        if args.len() != 5 || args[1] != "--input" || args[3] != "--out" {
            return Err("usage: work-engine-review-host prepare --input ABSOLUTE_JSON --out ABSOLUTE_NEW_DIRECTORY".into());
        }
        let result = review_workflow::preparation::prepare(
            std::path::Path::new(&args[2]),
            std::path::Path::new(&args[4]),
        )
        .map_err(|e| e.to_string())?;
        println!("{result}");
        return Ok(());
    }
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
