//! Explicit client entrypoint. It never launches a provider or reads auth state.
use anyhow::{Result, ensure};
use context_lifecycle_experiment::{
    Client, Clock, SystemClock,
    vertical::{TextVerticalConfig, run_text_vertical},
};
use std::net::SocketAddr;

fn validate_deadlines(config: &TextVerticalConfig, now_ms: u64) -> Result<()> {
    ensure!(
        config.authority.deadline > now_ms,
        "authority deadline must be a future Unix epoch millisecond timestamp"
    );
    ensure!(
        config.transition_deadline > now_ms,
        "transition deadline must be a future Unix epoch millisecond timestamp"
    );
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().collect();
    ensure!(
        arguments.len() == 3,
        "usage: native-text-vertical COORDINATOR_ADDRESS CONFIG_JSON"
    );
    let address: SocketAddr = arguments[1].parse()?;
    ensure!(
        address.ip().is_loopback(),
        "local coordinator address required"
    );
    let config: TextVerticalConfig = serde_json::from_slice(&std::fs::read(&arguments[2])?)?;
    validate_deadlines(&config, SystemClock.now())?;
    let result = run_text_vertical(&Client::new(address), config)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use context_lifecycle_experiment::{Mode, RuntimeId, SyntheticAuthority};

    fn config(authority_deadline: u64, transition_deadline: u64) -> TextVerticalConfig {
        TextVerticalConfig {
            first_input: vec![],
            second_input: vec![],
            first_fact: String::new(),
            second_fact: String::new(),
            continuation_objective: String::new(),
            limited_package_adequacy: String::new(),
            runtime: RuntimeId::new("test"),
            authority: SyntheticAuthority {
                approval: "test".into(),
                deadline: authority_deadline,
                mode: Mode::CodexTextLive,
            },
            transition_deadline,
            observation_timeout_ms: 0,
        }
    }

    #[test]
    fn rejects_seconds_and_expired_transition_before_client_use() {
        let now_ms = 1_791_350_664_000;
        let seconds = 1_791_350_664;
        assert!(
            validate_deadlines(&config(seconds, now_ms + 600_000), now_ms)
                .unwrap_err()
                .to_string()
                .contains("authority deadline")
        );
        assert!(
            validate_deadlines(&config(now_ms + 600_000, seconds), now_ms)
                .unwrap_err()
                .to_string()
                .contains("transition deadline")
        );
        validate_deadlines(&config(now_ms + 600_000, now_ms + 600_000), now_ms).unwrap();
    }
}
