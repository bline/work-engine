//! Explicit synthetic collector fixture. It never enters Docker or a provider.
use std::{fs, io::Write, path::Path, thread, time::Duration};
unsafe extern "C" {
    fn signal(sig: i32, handler: usize) -> usize;
}
fn journal(root: &Path, phase: &str, removed: bool) {
    let text = fs::read_to_string(root.join("journal.tpl"))
        .unwrap()
        .replace("@P@", phase)
        .replace("@R@", if removed { "true" } else { "false" });
    fs::write(root.join("state/pi-state.tmp"), text).unwrap();
    fs::rename(
        root.join("state/pi-state.tmp"),
        root.join("state/pi-state.json"),
    )
    .unwrap();
}
fn main() {
    let root = std::env::var("FIXTURE_ROOT").expect("explicit synthetic fixture root");
    let root = Path::new(&root);
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args[1], "pi-docker");
    match args[2].as_str() {
        "run" => {
            assert!(!root.join("state/pi-state.json").exists());
            let mut effect = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join("effects"))
                .unwrap();
            writeln!(effect, "entered").unwrap();
            effect.sync_all().unwrap();
            let mut runs = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join("runs"))
                .unwrap();
            writeln!(runs, "run").unwrap();
            journal(root, "running", false);
            if root.join("hang").exists() {
                unsafe {
                    signal(15, 1);
                }
                fs::write(root.join("owned-pid"), std::process::id().to_string()).unwrap();
                print!("partial-before-deadline\n");
                eprintln!("partial-stderr-before-deadline");
                std::io::stdout().flush().unwrap();
                loop {
                    thread::sleep(Duration::from_millis(50));
                }
            }
            thread::sleep(Duration::from_secs(2));
            journal(root, "collected", true);
            println!("{}", fs::read_to_string(root.join("receipt.json")).unwrap());
        }
        "inspect" => print!(
            "{}",
            fs::read_to_string(root.join("state/pi-state.json")).unwrap()
        ),
        "cancel" => {
            fs::write(root.join("cancel-marker"), b"requested").unwrap();
            println!("{{\"status\":\"cancellation_requested\"}}");
        }
        "reconcile" => {
            journal(root, "collected", true);
            println!("{}", fs::read_to_string(root.join("receipt.json")).unwrap());
        }
        _ => std::process::exit(64),
    }
}
