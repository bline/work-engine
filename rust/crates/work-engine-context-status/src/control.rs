//! Read-only, session-bound observations for manual handoff decisions.
//! The caller owns delivery and retains the returned cursor only after consuming
//! the result. This module never publishes to the board or transfers a session.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

const HELP: &str = "Usage: work-engine-context-status control --session ID --rollout FILE (--trigger-percent P --band-floor-percent P | --trigger-tokens N --band-floor-tokens N) [--step-percent S] [--host codex|claude] [--agent ID] [--lead-calls N] [--cursor JSON]\nPercent levels are shares of the context window recorded with each reading, so they follow the model's window. Claude transcripts record no window and need token levels. In band, another event is sent each time input crosses a further multiple of --step-percent of the window (default 5). Levels are caller-selected planning hypotheses, not reset limits. Output is JSON; no publication or handoff occurs.";

const CURSOR_VERSION: u32 = 2;
const DEFAULT_STEP_PERCENT: f64 = 5.0;

/// Where pressure levels sit. A token count fixed at launch stays put when the
/// model's window grows, so percent levels are resolved from each reading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Levels {
    WindowPercent { trigger: f64, band_floor: f64 },
    Tokens { trigger: u64, band_floor: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    host: String,
    session: String,
    agent: Option<String>,
    rollout: PathBuf,
    levels: Levels,
    step_percent: f64,
    lead_calls: u32,
}

/// Smallest input at or above `percent` of `window`.
fn share(window: u64, percent: f64) -> u64 {
    (window as f64 * percent / 100.0).ceil() as u64
}

fn percent_of(tokens: u64, window: u64) -> f64 {
    (tokens as f64 * 1000.0 / window as f64).round() / 10.0
}

fn parse_percent(value: &str, flag: &str) -> Result<f64, String> {
    let n: f64 = value.parse().map_err(|_| format!("invalid {flag}"))?;
    if !n.is_finite() || n <= 0.0 || n > 100.0 {
        return Err(format!("{flag} must be greater than 0 and at most 100"));
    }
    Ok(n)
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
    // In band, the multiple of step_percent reached; a new one is reported so
    // the listener does not go quiet for the rest of the window.
    step: Option<u64>,
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
    let window = raw["context_window_tokens"].as_u64().filter(|w| *w > 0);
    let resolved = match binding.levels {
        Levels::Tokens {
            trigger,
            band_floor,
        } => Some((trigger, band_floor)),
        Levels::WindowPercent {
            trigger,
            band_floor,
        } => window.map(|w| (share(w, trigger), share(w, band_floor))),
    };
    let failure = failure.or_else(|| {
        resolved.is_none().then(|| {
            "no context window recorded with the latest reading; percent levels cannot be resolved"
                .to_owned()
        })
    });
    let (trigger, band_floor) = resolved.unzip();
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
        .zip(band_floor)
        .filter(|_| {
            samples >= 3
                && delta > 0
                && current_valid
                && raw["freshness"] == "recent_file_modification"
        })
        .map(|(n, floor)| floor.saturating_sub(n) as f64 * (samples - 1) as f64 / delta as f64);
    let level = match (input, resolved) {
        (Some(n), Some((trigger, floor))) if current_valid => {
            if n >= floor {
                Level::InBand
            } else if calls_to_band.is_some_and(|n| n <= f64::from(binding.lead_calls)) {
                Level::ApproachingBand
            } else if n >= trigger {
                Level::Planning
            } else {
                Level::Normal
            }
        }
        _ => Level::Unavailable,
    };
    // Without a recorded window (Claude) there are no steps, only level changes.
    let step = match (&level, input, window) {
        (Level::InBand, Some(n), Some(w)) => {
            Some((n as f64 * 100.0 / w as f64 / binding.step_percent).floor() as u64)
        }
        _ => None,
    };
    let cursor = Cursor {
        version: CURSOR_VERSION,
        binding: binding.clone(),
        level: level.clone(),
        epoch,
        model,
        window: raw["context_window_tokens"].as_u64(),
        observation_status: status.into(),
        line: latest["line"].as_u64(),
        step,
    };
    let changed = previous.as_ref().map_or(level != Level::Normal, |old| {
        old.level != level
            || old.epoch != cursor.epoch
            || old.model != cursor.model
            || old.window != cursor.window
            || old.observation_status != cursor.observation_status
            || old.step != cursor.step
            || matches!((old.line, cursor.line), (Some(a), Some(b)) if b < a)
    });
    let mut topic = format!("control:session:{}", binding.session);
    if let Some(agent) = &binding.agent {
        topic.push_str(&format!("/agent:{agent}"));
    }
    let (basis, trigger_percent, band_floor_percent) = match binding.levels {
        Levels::WindowPercent {
            trigger,
            band_floor,
        } => ("window_percent", Some(trigger), Some(band_floor)),
        Levels::Tokens {
            trigger,
            band_floor,
        } => (
            "tokens",
            window.map(|w| percent_of(trigger, w)),
            window.map(|w| percent_of(band_floor, w)),
        ),
    };
    let next_step_percent = step
        .map(|s| (s + 1) as f64 * binding.step_percent)
        .filter(|p| *p <= 100.0);
    let event = changed.then(|| {
        let mut event = json!({
            "kind": "context_pressure", "topic": topic, "session_id": binding.session,
            "agent_id": binding.agent, "host": binding.host, "authority": "observation_only",
            "level": level, "input_tokens": input, "epoch": epoch,
            "recorded_model": cursor.model, "context_window_tokens": cursor.window,
            "trigger_tokens": trigger, "band_floor_tokens": band_floor,
            "headroom_to_band_tokens": input.zip(band_floor).map(|(n, f)| f.saturating_sub(n)),
            "estimated_calls_to_band": calls_to_band, "growth_sample_count": samples,
            "forecast_basis": "recent same-epoch endpoint growth; empirical band, not a reset deadline",
            "observed_at": latest["timestamp"], "source_line": cursor.line,
            "freshness": raw["freshness"], "observation_status": status,
            "limitations": raw["limitations"], "unavailable_detail": failure,
            "last_record_gap_line": raw["last_record_gap_line"],
        });
        // Inserted separately to stay inside json!'s macro recursion limit.
        let window_fields = json!({
            "level_basis": basis,
            "trigger_percent": trigger_percent, "band_floor_percent": band_floor_percent,
            "context_percent": input.zip(window).map(|(n, w)| percent_of(n, w)),
            "headroom_to_window_tokens": input.zip(window).map(|(n, w)| w.saturating_sub(n)),
            "step_percent": binding.step_percent, "next_step_percent": next_step_percent,
        });
        if let (Some(target), Value::Object(extra)) = (event.as_object_mut(), window_fields) {
            target.extend(extra);
        }
        event
    });
    json!({"schema": "manual-context-control-v1", "topic": topic,
        "event": event, "next_control_cursor": serde_json::to_string(&cursor).expect("cursor serializes")})
}

pub(crate) fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut host = "codex".to_owned();
    let mut session = String::new();
    let mut agent = None;
    let mut rollout = PathBuf::new();
    let mut lead_calls = 3;
    let (mut trigger_tokens, mut band_floor_tokens) = (None, None);
    let (mut trigger_percent, mut band_floor_percent) = (None, None);
    let mut step_percent = DEFAULT_STEP_PERCENT;
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
            "--host" => host = value,
            "--session" => session = value,
            "--agent" => agent = Some(value),
            "--rollout" => rollout = PathBuf::from(value),
            "--trigger-tokens" => trigger_tokens = Some(crate::parse_u64(&value, &flag)?),
            "--band-floor-tokens" => band_floor_tokens = Some(crate::parse_u64(&value, &flag)?),
            "--trigger-percent" => trigger_percent = Some(parse_percent(&value, &flag)?),
            "--band-floor-percent" => band_floor_percent = Some(parse_percent(&value, &flag)?),
            "--step-percent" => step_percent = parse_percent(&value, &flag)?,
            "--lead-calls" => lead_calls = value.parse().map_err(|_| "invalid --lead-calls")?,
            "--cursor" => cursor_text = Some(value),
            _ => return Err(format!("unknown control option {flag}")),
        }
    }
    let levels = match (
        trigger_tokens,
        band_floor_tokens,
        trigger_percent,
        band_floor_percent,
    ) {
        (Some(trigger), Some(band_floor), None, None) if trigger > 0 && band_floor > trigger => {
            Levels::Tokens {
                trigger,
                band_floor,
            }
        }
        (None, None, Some(_), Some(_)) if host == "claude" => {
            return Err("Claude transcripts record no context window; use --trigger-tokens and --band-floor-tokens".into());
        }
        (None, None, Some(trigger), Some(band_floor)) if band_floor > trigger => {
            Levels::WindowPercent {
                trigger,
                band_floor,
            }
        }
        _ => {
            return Err(format!(
                "give --trigger-percent and --band-floor-percent, or --trigger-tokens and --band-floor-tokens, with the trigger below the band floor; {HELP}"
            ));
        }
    };
    let binding = Binding {
        host,
        session,
        agent,
        rollout,
        levels,
        step_percent,
        lead_calls,
    };
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
            let value: Value =
                serde_json::from_str(text).map_err(|_| "invalid control cursor".to_owned())?;
            if value["version"]
                .as_u64()
                .is_some_and(|v| v != u64::from(CURSOR_VERSION))
            {
                return Err("control cursor is from another listener version; omit it to establish a new baseline".to_owned());
            }
            serde_json::from_value(value).map_err(|_| "invalid control cursor".to_owned())
        })
        .transpose()?;
    if previous.as_ref().is_some_and(|old| old.binding != binding) {
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
