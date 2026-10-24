//! Read-only, session-bound observations for manual handoff decisions.
//! The caller owns delivery and retains the returned cursor only after consuming
//! the result. This module never publishes to the board or transfers a session.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

const HELP: &str = "Usage: work-engine-context-status control --session ID --rollout FILE --trigger-tokens N --band-floor-tokens N [--host codex|claude] [--agent ID] [--lead-calls N] [--cursor JSON]\nLevels are caller-selected planning hypotheses, not reset limits. Output is JSON; no publication or handoff occurs.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    host: String,
    session: String,
    agent: Option<String>,
    rollout: PathBuf,
    trigger: u64,
    band_floor: u64,
    lead_calls: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Level {
    Normal,
    Planning,
    ApproachingBand,
    InBand,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u32,
    binding: Binding,
    level: Level,
    epoch: Option<u64>,
    model: Option<String>,
    window: Option<u64>,
    observation_status: String,
    // Bound replay to the same observed history; truncation must not leave a
    // stale high reading silently suppressing alerts from a replacement file.
    line: Option<u64>,
}

fn report(binding: &Binding) -> Result<Value, String> {
    let (mut raw, gap) = if binding.host == "claude" {
        let report = crate::claude::control_report(
            &binding.rollout,
            binding.session.clone(),
            binding.agent.clone(),
        )?;
        (serde_json::to_value(&report), report.last_record_gap_line)
    } else {
        let args = crate::args_for_child(binding.session.clone(), binding.rollout.clone());
        let report = crate::read_report(&args, &binding.rollout)?;
        (serde_json::to_value(&report), report.last_record_gap_line)
    };
    let value = raw.as_mut().map_err(|e| e.to_string())?;
    value["last_record_gap_line"] = json!(gap);
    raw.map_err(|e| e.to_string())
}

fn evaluate(
    binding: Binding,
    observation: Result<Value, String>,
    previous: Option<Cursor>,
) -> Value {
    let (raw, failure) = match observation {
        Ok(value) => (value, None),
        Err(error) => (Value::Null, Some(error)),
    };
    let claude = binding.host == "claude";
    let latest = &raw[if claude {
        "latest_call"
    } else {
        "latest_request"
    }];
    let input = latest[if claude {
        "context_tokens"
    } else {
        "input_tokens"
    }]
    .as_u64();
    let epoch = latest["epoch"].as_u64().or_else(|| raw["epoch"].as_u64());
    let model = raw[if claude {
        "latest_model"
    } else {
        "recorded_model"
    }]
    .as_str()
    .map(str::to_owned);
    let status = raw[if claude {
        "latest_call_status"
    } else {
        "latest_request_status"
    }]
    .as_str()
    .unwrap_or("usage_unavailable");
    let gap_line = raw["last_record_gap_line"].as_u64().unwrap_or(0);
    let current_valid = failure.is_none()
        && input.is_some_and(|n| n > 0)
        && latest["line"].as_u64().is_some_and(|line| line > gap_line);
    let input = input.filter(|_| current_valid);
    // Historical gaps remain in the reader's diagnostics, but cannot permanently
    // silence later valid observations. Forecast only from the valid suffix.
    let recent = raw[if claude {
        "recent_calls"
    } else {
        "recent_requests"
    }]
    .as_array()
    .into_iter()
    .flatten()
    .filter(|row| row["line"].as_u64().is_some_and(|line| line > gap_line))
    .filter_map(|row| {
        row[if claude {
            "context_tokens"
        } else {
            "input_tokens"
        }]
        .as_u64()
    })
    .collect::<Vec<_>>();
    let samples = recent.len();
    let delta = recent
        .last()
        .zip(recent.first())
        .map_or(0, |(last, first)| i128::from(*last) - i128::from(*first));
    // An extrapolation to an empirical band, never a prediction of when the
    // provider will compact. Need at least three observations in one epoch.
    let calls_to_band = input
        .filter(|_| {
            samples >= 3
                && delta > 0
                && current_valid
                && raw["freshness"] == "recent_file_modification"
        })
        .map(|n| binding.band_floor.saturating_sub(n) as f64 * (samples - 1) as f64 / delta as f64);
    let level = match input {
        None => Level::Unavailable,
        Some(_) if !current_valid => Level::Unavailable,
        Some(n) if n >= binding.band_floor => Level::InBand,
        Some(_) if calls_to_band.is_some_and(|n| n <= f64::from(binding.lead_calls)) => {
            Level::ApproachingBand
        }
        Some(n) if n >= binding.trigger => Level::Planning,
        Some(_) => Level::Normal,
    };
    let cursor = Cursor {
        version: 1,
        binding: binding.clone(),
        level: level.clone(),
        epoch,
        model,
        window: raw["context_window_tokens"].as_u64(),
        observation_status: status.into(),
        line: latest["line"].as_u64(),
    };
    let changed = previous.as_ref().map_or(level != Level::Normal, |old| {
        old.level != level
            || old.epoch != cursor.epoch
            || old.model != cursor.model
            || old.window != cursor.window
            || old.observation_status != cursor.observation_status
            || matches!((old.line, cursor.line), (Some(a), Some(b)) if b < a)
    });
    let mut topic = format!("control:session:{}", binding.session);
    if let Some(agent) = &binding.agent {
        topic.push_str(&format!("/agent:{agent}"));
    }
    let event = changed.then(|| json!({
        "kind": "context_pressure", "topic": topic, "session_id": binding.session,
        "agent_id": binding.agent, "host": binding.host, "authority": "observation_only",
        "level": level, "input_tokens": input, "epoch": epoch,
        "recorded_model": cursor.model, "context_window_tokens": cursor.window,
        "trigger_tokens": binding.trigger, "band_floor_tokens": binding.band_floor,
        "headroom_to_band_tokens": input.map(|n| binding.band_floor.saturating_sub(n)),
        "estimated_calls_to_band": calls_to_band, "growth_sample_count": samples,
        "forecast_basis": "recent same-epoch endpoint growth; empirical band, not a reset deadline",
        "observed_at": latest["timestamp"], "source_line": cursor.line,
        "freshness": raw["freshness"], "observation_status": status,
        "limitations": raw["limitations"], "unavailable_detail": failure,
        "last_record_gap_line": raw["last_record_gap_line"],
    }));
    json!({"schema": "manual-context-control-v1", "topic": topic,
        "event": event, "next_control_cursor": serde_json::to_string(&cursor).expect("cursor serializes")})
}

pub(crate) fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut binding = Binding {
        host: "codex".into(),
        session: String::new(),
        agent: None,
        rollout: PathBuf::new(),
        trigger: 0,
        band_floor: 0,
        lead_calls: 3,
    };
    let mut cursor_text = None;
    let mut args = args;
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!("{HELP}");
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--host" => binding.host = value,
            "--session" => binding.session = value,
            "--agent" => binding.agent = Some(value),
            "--rollout" => binding.rollout = PathBuf::from(value),
            "--trigger-tokens" => binding.trigger = crate::parse_u64(&value, &flag)?,
            "--band-floor-tokens" => binding.band_floor = crate::parse_u64(&value, &flag)?,
            "--lead-calls" => {
                binding.lead_calls = value.parse().map_err(|_| "invalid --lead-calls")?
            }
            "--cursor" => cursor_text = Some(value),
            _ => return Err(format!("unknown control option {flag}")),
        }
    }
    if !crate::valid_session_id(&binding.session)
        || !binding.rollout.is_absolute()
        || !matches!(binding.host.as_str(), "codex" | "claude")
        || (binding.host == "codex" && binding.agent.is_some())
        || binding.agent.as_ref().is_some_and(|id| {
            id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        || binding.trigger == 0
        || binding.band_floor <= binding.trigger
        || binding.lead_calls > 1000
    {
        return Err(format!("invalid control binding or levels; {HELP}"));
    }
    let previous: Option<Cursor> = cursor_text
        .as_ref()
        .map(|text| {
            if text.len() > 8192 {
                return Err("control cursor exceeds 8192 bytes".to_owned());
            }
            serde_json::from_str(text).map_err(|_| "invalid control cursor".to_owned())
        })
        .transpose()?;
    if previous
        .as_ref()
        .is_some_and(|old| old.version != 1 || old.binding != binding)
    {
        return Err("control cursor belongs to another binding or policy; omit it to establish a new baseline".into());
    }
    let observation = report(&binding);
    let value = evaluate(binding, observation, previous);
    if value["next_control_cursor"]
        .as_str()
        .is_some_and(|s| s.len() > 8192)
    {
        return Err(
            "generated control cursor exceeds 8192 bytes; binding cannot be retained".into(),
        );
    }
    println!("{value}");
    Ok(())
}
