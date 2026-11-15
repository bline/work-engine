//! Generate the selected-task presentation envelope from its Rust DTOs.
fn main() {
    let schema = schemars::schema_for!(operator_observation::selected_task::SelectedTask);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
