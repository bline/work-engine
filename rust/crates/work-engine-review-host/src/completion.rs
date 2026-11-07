//! Completion uses the same private startup loader and executable as review.
pub fn run(args: &[std::ffi::OsString], bootstrap: bool) -> Result<(), String> {
    let mut startup_args = args.to_vec();
    // Completion never performs job admission; the job/review owner must have
    // initialized the exact selected campaign before completion can reopen it.
    startup_args[0] = "serve".into();
    let (_, input) = crate::admission::StartupInput::parse(&startup_args)?;
    let startup = input.load(false).map_err(|e| e.to_string())?;
    let workflow = review_workflow::Workflow::open(startup).map_err(|e| e.to_string())?;
    if bootstrap {
        let result = workflow
            .initialize_completion()
            .map_err(|e| e.to_string())?;
        println!("{result}");
        Ok(())
    } else {
        workflow.serve_completion().map_err(|e| e.to_string())
    }
}
