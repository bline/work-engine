use lifecycle_core::{BuildId, ContextGeneration};

fn accept_context(_: ContextGeneration) {}

fn main() {
    let build = BuildId::parse("same-text").unwrap();
    accept_context(build);
}
