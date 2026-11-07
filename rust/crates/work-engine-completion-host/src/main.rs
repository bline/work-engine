//! Fixture-only executable; installation and route live in the library.
#[cfg(feature = "fixture")]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--fixture-config" {
        eprintln!("usage: work-engine-completion-host --fixture-config /absolute/installed.json");
        std::process::exit(2)
    }
    if let Err(e) =
        work_engine_completion_host::fixture_host::run_fixture(std::path::PathBuf::from(&args[2]))
    {
        eprintln!("{e}");
        std::process::exit(1)
    }
}

#[cfg(not(feature = "fixture"))]
fn main() {
    eprintln!(
        "fixture host is disabled; production completion is served by work-engine-review-host"
    );
    std::process::exit(2);
}
