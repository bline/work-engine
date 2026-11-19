//! `hosted-route-availability <config.json>`: print the route availability
//! view (JSON) for one operator configuration.
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        eprintln!("usage: hosted-route-availability <config.json>");
        std::process::exit(2);
    };
    let result = std::fs::read(path)
        .map_err(|e| format!("{path}: {e}"))
        .and_then(|b| hosted_route_availability::view(&b));
    match result {
        Ok(out) => std::io::stdout().write_all(&out).expect("stdout"),
        Err(e) => {
            eprintln!("hosted-route-availability: {e}");
            std::process::exit(2);
        }
    }
}
