use std::io::{stderr, stdout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = proposal_evaluation::cli::run(&args, &mut stdout().lock(), &mut stderr().lock());
    std::process::exit(code);
}
