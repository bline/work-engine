//! Local JSON fixture: policy, current identity, clock and an array of tagged records.
use context_pressure::Disposition;
use pi_context_observer::{Identity, Observation, Observer};
use serde_json::{Value, json};
use std::io::{self, Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    let input: Value = serde_json::from_str(&text)?;
    let identity: Identity = serde_json::from_value(input["identity"].clone())?;
    let mut observer = Observer::new(
        identity,
        input["max_age_ms"].as_u64().ok_or("missing max_age_ms")?,
        &input["policy"],
        Disposition::parse(
            input["initial_disposition"]
                .as_str()
                .ok_or("missing initial_disposition")?,
        )
        .ok_or("invalid initial_disposition")?,
    )?;
    let now = input["now_ms"].as_u64().ok_or("missing now_ms")?;
    let records = input["records"].as_array().ok_or("missing records")?;
    let mut out = Vec::new();
    for record in records {
        if record["kind"] == "pi" {
            let reading: Observation = serde_json::from_value(record["payload"].clone())?;
            out.push(match observer.observe(reading, now) {
                Ok(value) => json!({"kind":"pi","result":value}),
                Err(error) => json!({"kind":"pi","error":error}),
            });
        } else {
            out.push(record.clone());
        }
    }
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
