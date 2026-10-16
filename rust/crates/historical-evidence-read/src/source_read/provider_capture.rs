use super::{Result, array, decimal_sum, regular_read, sha, string, u64_field};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;

pub fn inspect(grant: &Value) -> Result<Value> {
    let provider = &grant["provider"];
    let captures = array(provider, "captures")?;
    let descriptors = array(provider, "results")?;
    if captures.len() != 2 || descriptors.len() != 8 {
        return Err("provider inventory is not exact".into());
    }
    let mut files = Vec::new();
    for (i, capture) in captures.iter().enumerate() {
        let path = Path::new(string(capture, "path")?);
        if !path.is_absolute() {
            return Err("capture path is not absolute".into());
        }
        let cap = if i == 0 {
            8 * 1024 * 1024
        } else {
            32 * 1024 * 1024
        };
        let bytes = regular_read(path, cap)?;
        if sha(&bytes) != string(capture, "sha256")? {
            return Err("provider source digest changed".into());
        }
        files.push(bytes);
    }
    let mut observed = Vec::new();
    let mut seen = HashSet::new();
    let mut costs = Vec::new();
    let mut total = [0u64; 5];
    let mut successes = 0u64;
    let mut errors = 0u64;
    for descriptor in descriptors {
        let file_index = u64_field(descriptor, "capture")? as usize;
        let bytes = files
            .get(file_index)
            .ok_or("provider capture index invalid")?;
        let line_number = u64_field(descriptor, "line")?;
        if line_number == 0 || line_number > 10000 {
            return Err("provider line outside bound".into());
        }
        let line = bytes
            .split(|b| *b == b'\n')
            .nth((line_number - 1) as usize)
            .ok_or("provider line is absent")?;
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.len() > 256 * 1024 || sha(line) != string(descriptor, "line_sha256")? {
            return Err("provider line size or digest changed".into());
        }
        let event: Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
        if string(&event, "type")? != "event_msg"
            || string(&event["payload"], "type")? != "item_completed"
        {
            return Err("provider source is not a completed event".into());
        }
        let item = &event["payload"]["item"];
        if string(item, "type")? != "CommandExecution"
            || string(item, "id")? != string(descriptor, "event_id")?
            || item["exit_code"].as_i64() != descriptor["exit_code"].as_i64()
        {
            return Err("provider execution identity differs".into());
        }
        let command = array(item, "command")?;
        let command_bytes = serde_json::to_vec(command).map_err(|e| e.to_string())?;
        if sha(&command_bytes) != string(descriptor, "command_sha256")? {
            return Err("provider command identity differs".into());
        }
        let command_kind = if command
            .iter()
            .any(|v| v.as_str().is_some_and(|s| s.contains("claude -p")))
        {
            "direct_claude_cli"
        } else {
            "historical_shell_replay"
        };
        if command_kind != string(descriptor, "command_kind")? {
            return Err("provider command class differs".into());
        }
        let stdout = string(item, "stdout")?;
        if stdout.len() > 65536 || sha(stdout.as_bytes()) != string(descriptor, "stdout_sha256")? {
            return Err("provider stdout size or digest changed".into());
        }
        let result: Value = serde_json::from_str(stdout).map_err(|e| e.to_string())?;
        let uuid = string(&result, "uuid")?;
        if uuid != string(descriptor, "uuid")? || !seen.insert(uuid.to_string()) {
            return Err("provider result UUID is repeated or changed".into());
        }
        let session = string(&result, "session_id")?;
        if session != string(descriptor, "session_id")? {
            return Err("provider session differs".into());
        }
        let is_error = result["is_error"]
            .as_bool()
            .ok_or("provider result status absent")?;
        let error_status = result["api_error_status"].as_u64();
        if is_error {
            errors += 1;
        } else {
            successes += 1;
        }
        if is_error
            != descriptor["is_error"]
                .as_bool()
                .ok_or("provider descriptor status absent")?
            || error_status != descriptor["api_error_status"].as_u64()
        {
            return Err("provider status differs".into());
        }
        let cost = result["total_cost_usd"]
            .as_number()
            .ok_or("provider cost absent")?
            .to_string();
        super::decimal_scaled(&cost)?;
        costs.push(cost.clone());
        let usage = &result["usage"];
        let usage_keys = [
            "input_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "output_tokens",
        ];
        let mut values = Vec::new();
        for key in usage_keys {
            values.push(u64_field(usage, key)?);
        }
        values.push(u64_field(
            &usage["output_tokens_details"],
            "thinking_tokens",
        )?);
        for (index, value) in values.iter().enumerate() {
            total[index] = total[index]
                .checked_add(*value)
                .ok_or("token sum overflow")?;
        }
        let model = result["modelUsage"]
            .as_object()
            .and_then(|m| m.keys().next())
            .cloned();
        observed.push(json!({
            "result_uuid":uuid,"session_id":session,"review_label":string(descriptor,"review_label")?,
            "event_id":string(descriptor,"event_id")?,"source_line":line_number,
            "source_file_sha256":string(&captures[file_index],"sha256")?,"source_line_sha256":string(descriptor,"line_sha256")?,
            "stdout_sha256":string(descriptor,"stdout_sha256")?,"command_kind":command_kind,
            "exit_code":item["exit_code"],"is_error":is_error,"api_error_status":error_status,
            "terminal_reason":string(&result,"terminal_reason")?,"model":model,
            "duration_ms":u64_field(&result,"duration_ms")?,"cost_usd":cost,
            "usage":{"input_tokens":values[0],"cache_creation_input_tokens":values[1],
                "cache_read_input_tokens":values[2],"output_tokens":values[3],"thinking_tokens":values[4]},
            "source_class":"captured_cli_result","attribution":"capture_descriptor_bound"
        }));
    }
    if successes != 7
        || errors != 1
        || observed
            .iter()
            .filter(|v| v["api_error_status"] == 429)
            .count()
            != 1
    {
        return Err("provider status inventory changed".into());
    }
    let sum = decimal_sum(&costs)?;
    Ok(
        json!({"source_owner":"slice-builder captured provider output","source_class":"captured_cli_result",
        "attempts":observed,"unique_result_uuids":seen.len(),"successes":successes,"errors":errors,
        "cost_literal_sum_usd":sum,
        "usage_total":{"input_tokens":total[0],"cache_creation_input_tokens":total[1],
            "cache_read_input_tokens":total[2],"output_tokens":total[3],"thinking_tokens":total[4]},
        "source_limit":"CLI result envelopes are captured output, not canonical provider billing or a complete workflow event stream"}),
    )
}
