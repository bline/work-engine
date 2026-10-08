//! External controlled child for evidence-owner process tests. It is not a
//! provider adapter or installed review service.
use std::fs::OpenOptions;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let count_path = arguments.next().ok_or("count path absent")?;
    let mode = arguments.next().unwrap_or_else(|| "echo".into());
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;
    let mut count = OpenOptions::new()
        .create(true)
        .append(true)
        .open(count_path)?;
    count.write_all(b"invoked\n")?;
    count.sync_all()?;
    match mode.as_str() {
        "echo" => {
            std::io::stdout().write_all(b"{\"request\":")?;
            std::io::stdout().write_all(&input)?;
            std::io::stdout().write_all(b"}\n")?;
            std::io::stderr().write_all(b"controlled-peer\n")?;
            Ok(())
        }
        "overflow" => {
            std::io::stdout().write_all(&vec![b'x'; 4 * 1024 * 1024 + 1])?;
            Ok(())
        }
        "exit1" => std::process::exit(1),
        "invalid" => {
            std::io::stdout().write_all(b"not-json")?;
            Ok(())
        }
        "sleep" => {
            std::thread::sleep(std::time::Duration::from_secs(10));
            Ok(())
        }
        _ => Err("unsupported controlled mode".into()),
    }
}
