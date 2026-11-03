//! Read-only projections of recorded evidence; no reader polling or acceptance.
use crate::{Operation, Phase, Result, SCHEMA, State, read_json, refuse};
use serde_json::{Value, json};
use std::path::Path;

fn fields(value: &Value, keys: &[&str]) -> Value {
    if !value.is_object() {
        return Value::Null;
    }
    Value::Object(
        keys.iter()
            .filter_map(|key| value.get(*key).map(|v| ((*key).to_string(), v.clone())))
            .collect(),
    )
}

pub(crate) fn operation_summary(state: &State, op: &Operation, dir: &Path) -> Value {
    let r = &op.request;
    let evidence = |key: &str| op.evidence.get(key).unwrap_or(&Value::Null);
    let settings = evidence("settings");
    let attributed = op.turn_id.is_some()
        && settings["turn_id"].as_str() == op.turn_id.as_deref()
        && settings["turn_attribution"] == "exact_turn_interval";
    let tier = if r.service_tier == "fast" {
        "priority"
    } else {
        &r.service_tier
    };
    let settings_status = if !attributed {
        "not_established"
    } else if settings["model"] == r.model
        && settings["reasoning_effort"] == r.effort
        && settings["service_tier"] == tier
    {
        "matched"
    } else {
        "mismatched"
    };
    let budget = &evidence("budget")["snapshot"];
    let controls = &budget["cost_controls"];
    let role_controls: Vec<_> = controls
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| op.session_id.is_some() && c["node_id"].as_str() == op.session_id.as_deref())
        .cloned()
        .collect();
    let realized = &evidence("prepared")["realized"];
    let receipt = op.receipt.as_ref().map(|id| {
        json!({
            "receipt_id": id,
            "path": dir.join("receipts").join(format!("{id}.json")),
            "delivery": if state.acknowledged.contains(id) { "acknowledged" } else { "pending" },
        })
    });
    json!({
        "operation_id": r.operation_id,
        "phase": op.phase,
        "evidence_status": "recorded_not_revalidated",
        "session_id": op.session_id, "turn_id": op.turn_id, "rollout": op.rollout,
        "identity": evidence("identity"),
        "turn_outcome": evidence("turn")["status"],
        "configuration": {
            "status": settings_status,
            "requested": {"model":r.model,"effort":r.effort,"service_tier":r.service_tier},
            "observed": fields(settings, &["model","reasoning_effort","service_tier","turn_id","turn_attribution","source"]),
        },
        "realization": {
            "requested_sandbox": r.sandbox, "require_read_only": r.require_read_only,
            "sandbox": realized["sandbox"],
            "approval_policy": realized["approvalPolicy"],
            "forked_from": realized["thread"]["forkedFromId"],
            "native_parent_thread": realized["thread"]["parentThreadId"],
            "field_presence": {
                "sandbox": realized.get("sandbox").is_some(),
                "forked_from": realized["thread"].get("forkedFromId").is_some(),
                "native_parent_thread": realized["thread"].get("parentThreadId").is_some(),
            },
            "evidence_scope": "recorded_thread_start; not wider tool isolation or current-turn grant",
        },
        "observations": {
            "root_context": evidence("root_context")["status"],
            "child_context": evidence("child_context")["status"],
            "child_context_established": op.context_established,
        },
        "context": {
            "root": fields(evidence("root_context"), &["status","input_tokens","context_window_tokens","observed_at","checked_at","freshness","epoch","recommendation","limitations"]),
            "child": fields(evidence("child_context"), &["status","input_tokens","context_window_tokens","observed_at","checked_at","freshness","epoch","recommendation","limitations"]),
        },
        "accounting": {
            "coverage": budget["status"], "snapshot_at": budget["snapshot_at"],
            "totals": budget["totals"], "post_close": budget["post_close"],
            "cost_controls": controls,
            "role_controls": role_controls,
            "role_control_status": if role_controls.is_empty() { "none_in_recorded_snapshot" } else { "configured_in_recorded_snapshot" },
            "account_quota": budget["measurements"]["account_quota"],
        },
        "receipt": receipt,
        "result": evidence("collection")["result"],
        "failures": op.failures,
        "acceptance": "root_owned", "integration": "root_owned",
        "monitoring": "not_established_by_recorded_state",
        "unknown_effects": {"create":matches!(op.phase,Phase::CreateUnknown),"start":matches!(op.phase,Phase::StartUnknown),"claim":matches!(op.claim,crate::Effect::Unknown),"budget_binding":matches!(op.budget_binding,crate::Effect::Unknown)},
        "processes": {
            "recorded": op.processes.len(),
            "without_exit": op.processes.iter().filter(|p|p.status.is_none()).count(),
            "timeouts": op.processes.iter().filter(|p|p.timed_out).count(),
            "recorded_leftovers": op.processes.iter().map(|p|p.leftovers.len()).sum::<usize>(),
            "evidence_scope": "recorded_process_status; current liveness not rechecked",
        },
    })
}

pub(crate) fn state_summary(state: &State, dir: &Path, operation: Option<&str>) -> Result<Value> {
    if operation.is_some_and(|id| !state.operations.contains_key(id)) {
        return refuse("unknown operation");
    }
    let operations: Vec<_> = state
        .operations
        .iter()
        .filter(|(id, _)| operation.is_none_or(|selected| selected == id.as_str()))
        .map(|(_, op)| operation_summary(state, op, dir))
        .collect();
    Ok(json!({
        "schema":"work-engine.manual-host.inspect.v1",
        "evidence_status":"recorded_state_not_refreshed",
        "objective_id":state.objective_id,"assignment":state.assignment,
        "binary_sha256":state.binary_sha256,"operations":operations,
    }))
}

/// Read one atomically replaced state file. Works across executor versions.
pub fn inspect(dir: &Path, operation: Option<&str>, summary: bool) -> Result<Value> {
    if !summary {
        if operation.is_some() {
            return refuse("--operation requires inspect --summary");
        }
        // Preserve the raw escape hatch even for oversized/other-version state
        // that the bounded executor and typed summary cannot open.
        return Ok(serde_json::from_slice(&std::fs::read(
            dir.join("state.json"),
        )?)?);
    }
    let raw = read_json(&dir.join("state.json"))?;
    if raw["schema"] != SCHEMA {
        return refuse("unsupported inspection state schema");
    }
    state_summary(&serde_json::from_value::<State>(raw)?, dir, operation)
}
