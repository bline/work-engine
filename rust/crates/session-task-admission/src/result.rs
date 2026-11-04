//! Conservative result projection before a worker contract is established.
use crate::Ref;
use serde::Serialize;

#[derive(Serialize)]
pub struct Clock<'a> {
    pub first_funded_at: Option<&'a str>,
    pub admitted_at: Option<&'a str>,
    pub observed_start_at: Option<&'a str>,
    pub close_by: &'a str,
}
#[derive(Serialize)]
pub struct Accounting<'a> {
    pub units: &'a str,
    pub limit: &'a str,
    pub reservation: Option<&'a str>,
    pub observed_usage: Option<&'a str>,
    pub coverage: &'a str,
    pub unsettled: Option<&'a str>,
}
#[derive(Serialize)]
pub struct Disposition<'a> {
    pub id: Option<&'a str>,
    pub disposition: &'a str,
}
#[derive(Serialize)]
pub struct Custody<'a> {
    pub native: Disposition<'a>,
    pub children: Vec<Disposition<'a>>,
    pub processes: Vec<Disposition<'a>>,
}
#[derive(Serialize)]
pub struct ResultView<'a> {
    schema: &'static str,
    task: &'a Ref,
    admission_id: &'a str,
    session_id: Option<&'a str>,
    turn_id: Option<&'a str>,
    worker_contract_revision: Option<u64>,
    outcome: Option<&'a str>,
    operational_status: &'static str,
    exceptions: Vec<String>,
    artifacts: Vec<Ref>,
    evidence: Vec<Ref>,
    clock: Clock<'a>,
    accounting: Accounting<'a>,
    custody: Custody<'a>,
    observed_at: &'a str,
    source_revision: &'a str,
}
/// Return an unresolved result, never a worker success or closure claim. A
/// caller must source identities/times/accounting from protected observations;
/// this does not validate or settle them. No worker result schema is wired yet.
#[allow(clippy::too_many_arguments)]
pub fn unresolved<'a>(
    task: &'a Ref,
    admission_id: &'a str,
    session_id: Option<&'a str>,
    turn_id: Option<&'a str>,
    _packet_present: bool,
    clock: Clock<'a>,
    accounting: Accounting<'a>,
    custody: Custody<'a>,
    observed_at: &'a str,
    source_revision: &'a str,
) -> ResultView<'a> {
    ResultView {
        schema: "dot.result-view/v1-proposal",
        task,
        admission_id,
        session_id,
        turn_id,
        worker_contract_revision: None,
        outcome: None,
        operational_status: "missing_or_invalid_worker_contract",
        exceptions: vec!["worker_contract_unestablished".into()],
        artifacts: vec![],
        evidence: vec![],
        clock,
        accounting,
        custody,
        observed_at,
        source_revision,
    }
}

/// Compose an observed worker outcome only from a packet already bound to an
/// independently selected turn. Accounting and custody remain separate inputs;
/// a worker outcome never settles either one.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn observed<'a>(
    task: &'a Ref,
    admission_id: &'a str,
    session_id: &'a str,
    turn_id: &'a str,
    packet: &'a CheckedPacket,
    provenance: Ref,
    clock: Clock<'a>,
    accounting: Accounting<'a>,
    custody: Custody<'a>,
    observed_at: &'a str,
    source_revision: &'a str,
) -> Option<ResultView<'a>> {
    if admission_id.is_empty()
        || session_id.is_empty()
        || turn_id.is_empty()
        || !crate::valid_ref(&provenance)
    {
        return None;
    }
    let mut evidence = packet.evidence.clone();
    if !evidence.contains(&provenance) {
        evidence.push(provenance);
    }
    Some(ResultView {
        schema: "dot.result-view/v1-proposal",
        task,
        admission_id,
        session_id: Some(session_id),
        turn_id: Some(turn_id),
        worker_contract_revision: Some(1),
        outcome: Some(&packet.outcome),
        operational_status: "worker_contract_observed",
        exceptions: packet.exceptions.clone(),
        artifacts: packet.artifacts.clone(),
        evidence,
        clock,
        accounting,
        custody,
        observed_at,
        source_revision,
    })
}

/// The packet binds to a task and admission. Session/turn binding must come
/// from a separately observed selected-turn adapter, not this worker document.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerPacket {
    schema: String,
    task: Ref,
    admission_id: String,
    worker_contract_revision: u64,
    outcome: String,
    artifacts: Vec<Ref>,
    evidence: Vec<Ref>,
    exceptions: Vec<String>,
}
#[allow(dead_code)]
pub(crate) struct CheckedPacket {
    pub(crate) outcome: String,
    pub(crate) artifacts: Vec<Ref>,
    pub(crate) evidence: Vec<Ref>,
    pub(crate) exceptions: Vec<String>,
}
/// Parsing alone is not producer verification. The native adapter must supply
/// bytes from the exact independently observed selected session and turn.
#[allow(dead_code)]
pub(crate) fn packet(raw: &str, task: &Ref, admission_id: &str) -> Option<CheckedPacket> {
    let p: WorkerPacket = serde_json::from_str(raw).ok()?;
    if p.schema != "dot.worker-result/v1-proposal"
        || p.task != *task
        || p.admission_id != admission_id
        || admission_id.is_empty()
        || p.worker_contract_revision != 1
        || ![
            "completed",
            "partial",
            "blocked",
            "failed",
            "interrupted",
            "limit",
            "cancelled",
        ]
        .contains(&p.outcome.as_str())
        || p.artifacts
            .iter()
            .chain(&p.evidence)
            .any(|r| !crate::valid_ref(r))
        || p.exceptions.iter().any(|s| !crate::nonempty(s))
    {
        return None;
    }
    Some(CheckedPacket {
        outcome: p.outcome,
        artifacts: p.artifacts,
        evidence: p.evidence,
        exceptions: p.exceptions,
    })
}

/// Constructed only by a future trusted native-custody observation adapter.
/// Worker-controlled JSON must never provide this identity or provenance.
#[allow(dead_code)]
pub(crate) struct SelectedTurn<'a> {
    pub(crate) session_id: &'a str,
    pub(crate) turn_id: &'a str,
    pub(crate) provenance: &'a Ref,
    pub(crate) packet_json: &'a str,
}
#[allow(dead_code)]
pub(crate) fn parse_selected(
    observed: &SelectedTurn<'_>,
    expected_session: &str,
    expected_turn: &str,
    task: &Ref,
    admission_id: &str,
) -> Option<CheckedPacket> {
    if expected_session.is_empty()
        || expected_turn.is_empty()
        || observed.session_id != expected_session
        || observed.turn_id != expected_turn
        || !crate::valid_ref(observed.provenance)
    {
        return None;
    }
    packet(observed.packet_json, task, admission_id)
}

/// Parse the existing bridge's correlated terminal observation. The caller
/// must obtain these bytes directly from the pinned bridge/session-events
/// adapter; worker-authored JSON is never the provenance source.
#[allow(dead_code)]
pub(crate) fn from_bridge_observation(
    raw: &str,
    expected_session: &str,
    expected_turn: &str,
    provenance: &Ref,
    task: &Ref,
    admission_id: &str,
) -> Option<CheckedPacket> {
    #[derive(serde::Deserialize)]
    struct Operation {
        session_id: String,
        native_turn_id: String,
        entry_state: String,
        native_status: String,
    }
    #[derive(serde::Deserialize)]
    struct Completion {
        available: bool,
        session_id: String,
        turn_id: String,
        status: String,
        entry_id: String,
    }
    #[derive(serde::Deserialize)]
    struct ResultText {
        available: bool,
        text: String,
        text_offset: usize,
        text_extent: usize,
        redaction_count: u64,
        next_offset: Option<usize>,
        truncated: bool,
    }
    #[derive(serde::Deserialize)]
    struct Observation {
        operation: Operation,
        completion: Completion,
        result: ResultText,
    }
    let o: Observation = serde_json::from_str(raw).ok()?;
    if o.operation.entry_state != "terminal"
        || !o.completion.available
        || !o.result.available
        || o.operation.session_id != expected_session
        || o.operation.native_turn_id != expected_turn
        || o.completion.session_id != expected_session
        || o.completion.turn_id != expected_turn
        || o.operation.native_status != o.completion.status
        || o.completion.entry_id.is_empty()
        || o.result.text_offset != 0
        || o.result.text_extent != o.result.text.chars().count()
        || o.result.redaction_count != 0
        || o.result.truncated
        || o.result.next_offset.is_some()
    {
        return None;
    }
    let selected = SelectedTurn {
        session_id: &o.operation.session_id,
        turn_id: &o.operation.native_turn_id,
        provenance,
        packet_json: &o.result.text,
    };
    parse_selected(
        &selected,
        expected_session,
        expected_turn,
        task,
        admission_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn worker_packet_requires_exact_task_and_admission_and_known_schema() {
        let task = Ref {
            id: "task".into(),
            revision: "r1".into(),
            sha256: "0".repeat(64),
        };
        let mut v = json!({"schema":"dot.worker-result/v1-proposal","task":task,"admission_id":"a","worker_contract_revision":1,"outcome":"completed","artifacts":[],"evidence":[],"exceptions":[]});
        assert_eq!(
            packet(&v.to_string(), &task, "a").unwrap().outcome,
            "completed"
        );
        let provenance = Ref {
            id: "native-observation".into(),
            revision: "r1".into(),
            sha256: "1".repeat(64),
        };
        let bytes = v.to_string();
        let observed = SelectedTurn {
            session_id: "session",
            turn_id: "turn",
            provenance: &provenance,
            packet_json: &bytes,
        };
        assert!(parse_selected(&observed, "session", "turn", &task, "a").is_some());
        assert!(parse_selected(&observed, "other", "turn", &task, "a").is_none());
        assert!(parse_selected(&observed, "session", "other", &task, "a").is_none());
        let mut observation = json!({"operation":{"session_id":"session","native_turn_id":"turn","entry_state":"terminal","native_status":"completed"},"completion":{"available":true,"session_id":"session","turn_id":"turn","status":"completed","entry_id":"e"},"result":{"available":true,"text":bytes,"text_offset":0,"text_extent":bytes.chars().count(),"redaction_count":0,"next_offset":null,"truncated":false}});
        assert!(
            from_bridge_observation(
                &observation.to_string(),
                "session",
                "turn",
                &provenance,
                &task,
                "a"
            )
            .is_some()
        );
        observation["completion"]["turn_id"] = "other".into();
        assert!(
            from_bridge_observation(
                &observation.to_string(),
                "session",
                "turn",
                &provenance,
                &task,
                "a"
            )
            .is_none()
        );
        observation["completion"]["turn_id"] = "turn".into();
        observation["result"]["redaction_count"] = 1.into();
        assert!(
            from_bridge_observation(
                &observation.to_string(),
                "session",
                "turn",
                &provenance,
                &task,
                "a"
            )
            .is_none()
        );
        v["task"]["revision"] = "r2".into();
        assert!(packet(&v.to_string(), &task, "a").is_none());
        v["task"]["revision"] = "r1".into();
        v["outcome"] = "invented".into();
        assert!(packet(&v.to_string(), &task, "a").is_none());
        v["outcome"] = "completed".into();
        v["new_field"] = json!(true);
        assert!(packet(&v.to_string(), &task, "a").is_none());
    }
}
