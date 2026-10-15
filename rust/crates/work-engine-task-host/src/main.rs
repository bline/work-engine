fn main() {
    if let Err(error) = work_engine_task_host::run() {
        eprintln!("task host failed: {error}");
        std::process::exit(1);
    }
}
