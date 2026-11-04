//! Explicit SDK observation profile. No task/inbox/lifecycle semantics live here.
use super::*;
pub const PROFILE: &str = "work-engine.pi-observation-sdk.v1";
pub const SHIM: &[u8] = include_bytes!("../receiver/pi-observation.mjs");
const CUSTOM: &str = "work-engine.delivery-observation.v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub shim: Artifact,
    /// Exact pre-authorized Event DTO, retaining original raw_payload bytes.
    pub event: Artifact,
    /// Existing delivery-v1 first attempt, derived only after native binding.
    pub attempt_number: u32,
    /// Exact existing recipient Binding except session, assigned after native readiness.
    pub recipient_without_session: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub schema: String,
    pub input_digest: String,
    pub container_id: String,
    pub native_session_id: String,
    pub envelope: Artifact,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Observation {
    pub process_index: Option<usize>,
    pub native_session_id: Option<String>,
    pub release_intent: Option<Artifact>,
    pub transport_accepted: bool,
    pub target_observed: bool,
    pub completed_stdout_sha256: Option<String>,
    pub native_history: Option<Artifact>,
}
impl Profile {
    pub(super) fn validate(&self, job: &Job) -> Result<()> {
        if !self.shim.path.is_absolute()
            || fs::canonicalize(&self.shim.path)? != self.shim.path
            || self.shim.path.to_string_lossy().contains([',', ':'])
            || !fs::symlink_metadata(&self.shim.path)?.is_file()
        {
            return refuse("receiver shim mount path invalid");
        }
        if self.schema != PROFILE
            || self.shim.verify()? != SHIM
            || self.attempt_number != 1
            || self.recipient_without_session.get("session").is_some()
        {
            return refuse("receiver profile/shim/identity conflict");
        }
        let b = &self.recipient_without_session;
        let keys = [
            "recipient",
            "principal",
            "job",
            "lane",
            "workspace",
            "generation",
            "address",
            "fixture_mode",
            "expires_at",
            "active",
        ];
        if b.as_object()
            .is_none_or(|m| m.len() != keys.len() || keys.iter().any(|k| !m.contains_key(*k)))
            || b["job"] != job.job_id
            || b["generation"].as_u64().is_none_or(|n| n == 0)
            || b["active"] != true
            || b["expires_at"]
                .as_u64()
                .is_none_or(|t| t < job.deadline_unix_seconds)
            || b["fixture_mode"] != "observe"
            || ["recipient", "principal", "lane", "workspace", "address"]
                .iter()
                .any(|k| b[*k].as_str().is_none_or(str::is_empty))
        {
            return refuse("receiver recipient constraint invalid");
        }
        let event: Value = serde_json::from_slice(&self.event.verify()?)?;
        let raw = event["raw_payload"]
            .as_str()
            .ok_or_else(|| Error::Refused("missing native event payload".into()))?;
        if raw.len() > 65536
            || event["payload_sha256"] != digest(raw.as_bytes())
            || event["event_id"].as_str().is_none_or(|s| !pin_is_hash(s))
        {
            return refuse("authorized event invalid");
        }
        Ok(())
    }
}
fn frames(bytes: &[u8], complete: bool) -> Result<Vec<Value>> {
    if bytes.len() as u64 > LIMIT || (complete && !bytes.ends_with(b"\n")) {
        return refuse("receiver framing bound/tail conflict");
    }
    let mut rows = vec![];
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        if !line.ends_with(b"\n") {
            break;
        }
        let value: Value = serde_json::from_slice(line)?;
        if value["schema"] != PROFILE {
            return refuse("receiver protocol/profile conflict");
        }
        rows.push(value);
    }
    Ok(rows)
}
fn ready(rows: &[Value]) -> Result<Option<String>> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let s = &first["state"];
    if first["type"] != "ready"
        || s["sessionId"].as_str().is_none_or(str::is_empty)
        || s["isStreaming"] != false
        || s["isCompacting"] != false
        || s["pendingMessageCount"] != 0
        || s["autoCompactionEnabled"] != false
        || s["autoRetryEnabled"] != false
    {
        return refuse("receiver readiness unavailable or busy");
    }
    if rows.iter().skip(1).any(|v| {
        v["type"] == "ready" || v["type"] == "error" || v["type"] == "forbidden_native_event"
    }) {
        return refuse("receiver error/replacement/forbidden event");
    }
    Ok(Some(s["sessionId"].as_str().unwrap().into()))
}
impl Host {
    pub(super) fn receiver_advance(
        &mut self,
        bytes: &[u8],
        number: usize,
    ) -> Result<Option<Vec<u8>>> {
        let rows = frames(bytes, false)?;
        if !bytes.ends_with(b"\n") {
            return Ok(None);
        }
        let Some(session) = ready(&rows)? else {
            return Ok(None);
        };
        let state = self
            .journal
            .receiver
            .as_ref()
            .ok_or_else(|| Error::Refused("receiver state absent".into()))?;
        if state
            .native_session_id
            .as_ref()
            .is_some_and(|s| s != &session)
            || state.process_index.is_some_and(|n| n != number)
        {
            return refuse("receiver process/session changed");
        }
        if state.native_session_id.is_none() {
            let state = self.journal.receiver.as_mut().unwrap();
            state.native_session_id = Some(session.clone());
            state.process_index = Some(number);
            self.journal.session_id = Some(session.clone());
            self.save()?;
            atomic(
                &self.journal.job.state_dir.join("receiver-ready.json"),
                &json!({"schema":PROFILE,"input_digest":self.journal.input_digest,"container_id":self.journal.container_id,"native_session_id":session,"deadline_unix_seconds":self.journal.job.deadline_unix_seconds}),
            )?;
        }
        if self
            .journal
            .receiver
            .as_ref()
            .unwrap()
            .release_intent
            .is_some()
        {
            return Ok(None);
        }
        if rows.len() != 1 {
            return refuse("unexpected native activity before release");
        }
        if !bytes.ends_with(b"\n") {
            return Ok(None);
        }
        let release_path = self.journal.job.state_dir.join("receiver-release.json");
        if !release_path.try_exists()? {
            return Ok(None);
        }
        if fs::symlink_metadata(&release_path)?
            .file_type()
            .is_symlink()
        {
            return refuse("receiver release symlink refused");
        }
        let release: Release = serde_json::from_slice(&crate::bytes(&release_path)?)?;
        if release.schema != PROFILE
            || release.input_digest != self.journal.input_digest
            || Some(&release.container_id) != self.journal.container_id.as_ref()
            || release.native_session_id != session
            || self.expired()
            || self.cancelled()
        {
            return refuse("receiver release job/container/session/limit conflict");
        }
        let raw = release.envelope.verify()?;
        let envelope: Value = serde_json::from_slice(&raw)?;
        let profile = self.journal.job.receiver.as_ref().unwrap();
        validate_envelope(profile, &envelope, &session)?;
        let snapshot = release
            .envelope
            .snapshot(&self.journal.job.state_dir.join("snapshots"))?;
        let pin = Artifact {
            path: snapshot,
            sha256: release.envelope.sha256.clone(),
        };
        self.journal.artifacts.push(pin.clone());
        // This durable fact precedes the only possible write. Recovery never resends.
        self.journal.receiver.as_mut().unwrap().release_intent = Some(pin);
        self.save()?;
        let mut command = serde_json::to_vec(
            &json!({"type":"observe","nativeSessionId":session,"envelopeSha256":release.envelope.sha256,"rawEnvelope":String::from_utf8(raw).map_err(|_|Error::Refused("receiver envelope must be UTF8".into()))?}),
        )?;
        command.push(b'\n');
        if command.len() > 131072 {
            return refuse("receiver release oversized");
        }
        Ok(Some(command))
    }
    pub(super) fn receiver_progress(&mut self, bytes: &[u8]) -> Result<()> {
        let rows = frames(bytes, false)?;
        let state = self.journal.receiver.as_ref().unwrap();
        let (Some(session), Some(pin)) = (&state.native_session_id, &state.release_intent) else {
            return Ok(());
        };
        let admitted = rows
            .iter()
            .filter(|v| v["type"] == "admitted")
            .collect::<Vec<_>>();
        if admitted.len() == 1
            && admitted[0]["sessionId"] == *session
            && admitted[0]["envelopeSha256"] == pin.sha256
            && !state.transport_accepted
        {
            self.journal.receiver.as_mut().unwrap().transport_accepted = true;
            self.save()?;
        }
        Ok(())
    }
    pub(super) fn receiver_complete(&mut self, bytes: &[u8], number: usize) -> Result<()> {
        let rows = frames(bytes, true)?;
        let session =
            ready(&rows)?.ok_or_else(|| Error::Refused("no receiver readiness".into()))?;
        let state = self.journal.receiver.as_ref().unwrap();
        if state.process_index != Some(number)
            || state.native_session_id.as_deref() != Some(&session)
        {
            return refuse("receiver completed process/session conflict");
        }
        let pin = state
            .release_intent
            .as_ref()
            .ok_or_else(|| Error::Refused("no receiver release intent".into()))?;
        let raw =
            String::from_utf8(pin.verify()?).map_err(|_| Error::Refused("receiver UTF8".into()))?;
        let history = verify_observation(&rows, &session, &raw, &pin.sha256)?;
        let path = self
            .journal
            .job
            .state_dir
            .join("exports/receiver-native-history.json");
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return refuse("native history symlink refused");
        }
        let artifact = Artifact {
            path,
            sha256: history,
        };
        let native: Value = serde_json::from_slice(&artifact.verify()?)?;
        if native["schema"] != PROFILE
            || native["sessionId"] != session
            || native["header"]["id"] != session
            || native["messages"]
                != rows.iter().find(|v| v["type"] == "get_messages").unwrap()["messages"]
            || !native["entries"].as_array().is_some_and(|e| {
                e.iter().any(|e| {
                    e["type"] == "custom_message"
                        && e["customType"] == CUSTOM
                        && e["details"]["rawEnvelope"] == raw
                        && e["details"]["envelopeSha256"] == pin.sha256
                })
            })
        {
            return refuse("native history export does not corroborate insertion");
        }
        if let Some(file) = native["nativeSessionFile"].as_str() {
            let relative = file
                .strip_prefix("/export/")
                .ok_or_else(|| Error::Refused("native session path outside exports".into()))?;
            let path = self.journal.job.state_dir.join("exports").join(relative);
            if !fs::canonicalize(&path)?.starts_with(fs::canonicalize(
                self.journal.job.state_dir.join("exports"),
            )?) || !fs::symlink_metadata(&path)?.is_file()
            {
                return refuse("native session file claim unestablished");
            }
            file_digest(&path, None)?;
        } else if !native["nativeSessionFile"].is_null() {
            return refuse("invalid native session file evidence");
        }
        let snap = artifact.snapshot(&self.journal.job.state_dir.join("snapshots"))?;
        let artifact = Artifact {
            path: snap,
            sha256: artifact.sha256,
        };
        self.journal.artifacts.push(artifact.clone());
        let state = self.journal.receiver.as_mut().unwrap();
        state.transport_accepted = true;
        state.target_observed = true;
        state.completed_stdout_sha256 = Some(digest(bytes));
        state.native_history = Some(artifact);
        self.save()
    }
}
fn validate_envelope(p: &Profile, e: &Value, session: &str) -> Result<()> {
    let keys = [
        "schema",
        "fixture_only",
        "authority",
        "delivery_id",
        "attempt_id",
        "subscription_id",
        "binding_sha256",
        "recipient",
        "event",
    ];
    if e.as_object()
        .is_none_or(|m| m.len() != keys.len() || keys.iter().any(|k| !m.contains_key(*k)))
        || e["schema"] != "work-engine.delivery-envelope/v1"
        || e["authority"] != "observation_only"
        || e["fixture_only"] != true
        || e["recipient"]["session"] != session
        || e["subscription_id"] != p.recipient_without_session["recipient"]
        || e["binding_sha256"].as_str().is_none_or(|h| !pin_is_hash(h))
        || e["event"] != serde_json::from_slice::<Value>(&p.event.verify()?)?
    {
        return refuse("receiver immutable envelope conflict");
    }
    let mut binding = e["recipient"].clone();
    binding
        .as_object_mut()
        .ok_or_else(|| Error::Refused("binding object required".into()))?
        .remove("session");
    if binding != p.recipient_without_session {
        return refuse("receiver principal/generation/binding conflict");
    }
    // Existing Binding has a named Serde field order; use that exact order for
    // its v1 digest rather than inventing a global canonical JSON encoding.
    let b = &e["recipient"];
    let wire = format!(
        "{{\"recipient\":{},\"principal\":{},\"job\":{},\"lane\":{},\"session\":{},\"workspace\":{},\"generation\":{},\"address\":{},\"fixture_mode\":{},\"expires_at\":{},\"active\":{}}}",
        b["recipient"],
        b["principal"],
        b["job"],
        b["lane"],
        b["session"],
        b["workspace"],
        b["generation"],
        b["address"],
        b["fixture_mode"],
        b["expires_at"],
        b["active"]
    );
    let binding_sha = digest(wire.as_bytes());
    let delivery_id = digest(&serde_json::to_vec(&(
        e["event"]["event_id"].as_str().unwrap(),
        b["recipient"].as_str().unwrap(),
        b["generation"].as_u64().unwrap(),
        &binding_sha,
    ))?);
    let attempt_id = digest(&serde_json::to_vec(&(&delivery_id, p.attempt_number))?);
    if e["delivery_id"] != delivery_id || e["attempt_id"] != attempt_id {
        return refuse("receiver delivery-v1 derived identity conflict");
    }
    if e["binding_sha256"] != digest(wire.as_bytes()) {
        return refuse("receiver v1 binding digest conflict");
    }
    Ok(())
}
fn verify_observation(rows: &[Value], session: &str, raw: &str, sha: &str) -> Result<String> {
    let mut admitted = None;
    let mut started = None;
    let mut ended = None;
    let mut messages = None;
    let mut history = None;
    let matches = |m: &Value| {
        m["role"] == "custom"
            && m["customType"] == CUSTOM
            && m["content"] == json!([{"type":"text","text":raw}])
            && m["display"] == false
            && m["details"] == json!({"rawEnvelope":raw,"envelopeSha256":sha})
    };
    for (i, row) in rows.iter().enumerate().skip(1) {
        if row["sessionId"] != session {
            return refuse("native evidence session conflict");
        }
        match row["type"].as_str() {
            Some("admitted") if admitted.is_none() && row["envelopeSha256"] == sha => {
                admitted = Some(i)
            }
            Some("native_event")
                if row["event"]["type"] == "message_start"
                    && started.is_none()
                    && admitted.is_some()
                    && ended.is_none()
                    && matches(&row["event"]["message"]) =>
            {
                started = Some(i);
            }
            Some("native_event")
                if row["event"]["type"] == "message_end"
                    && ended.is_none()
                    && matches(&row["event"]["message"]) =>
            {
                ended = Some(i)
            }
            Some("get_messages")
                if messages.is_none()
                    && row["messages"]
                        .as_array()
                        .is_some_and(|m| m.len() == 1 && matches(&m[0]))
                    && row["state"]["sessionId"] == session
                    && row["state"]["isStreaming"] == false
                    && row["state"]["isCompacting"] == false
                    && row["state"]["pendingMessageCount"] == 0
                    && row["state"]["autoCompactionEnabled"] == false
                    && row["state"]["autoRetryEnabled"] == false =>
            {
                messages = Some(i)
            }
            Some("history_export")
                if history.is_none()
                    && row["path"] == "receiver-native-history.json"
                    && row["sha256"].as_str().is_some_and(pin_is_hash) =>
            {
                history = Some((i, row["sha256"].as_str().unwrap().to_owned()))
            }
            _ => return refuse("native evidence unknown/duplicate/error/forbidden frame"),
        }
    }
    match (admitted, ended, messages, history) {
        (Some(a), Some(e), Some(m), Some((h, hash))) if a < e && e < m && m < h => Ok(hash),
        _ => refuse("native observation evidence missing or out of order"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn proof() -> Vec<Value> {
        let raw = "{ \"raw\": \"unavailable λ\" }\n";
        let sha = digest(raw.as_bytes());
        let state = json!({"sessionId":"native","isStreaming":false,"isCompacting":false,"pendingMessageCount":0,"autoCompactionEnabled":false,"autoRetryEnabled":false});
        let message = json!({"role":"custom","customType":CUSTOM,"content":[{"type":"text","text":raw}],"display":false,"details":{"rawEnvelope":raw,"envelopeSha256":sha}});
        vec![
            json!({"schema":PROFILE,"type":"ready","state":state}),
            json!({"schema":PROFILE,"type":"admitted","sessionId":"native","envelopeSha256":sha}),
            json!({"schema":PROFILE,"type":"native_event","sessionId":"native","event":{"type":"message_end","message":message}}),
            json!({"schema":PROFILE,"type":"get_messages","sessionId":"native","messages":[message],"state":state}),
            json!({"schema":PROFILE,"type":"history_export","sessionId":"native","path":"receiver-native-history.json","sha256":"f".repeat(64)}),
        ]
    }
    #[test]
    fn native_proof_requires_both_independent_views_exact_bytes_and_order() {
        let rows = proof();
        let raw = "{ \"raw\": \"unavailable λ\" }\n";
        let sha = digest(raw.as_bytes());
        assert!(verify_observation(&rows, "native", raw, &sha).is_ok());
        for removed in 1..5 {
            let mut bad = rows.clone();
            bad.remove(removed);
            assert!(verify_observation(&bad, "native", raw, &sha).is_err());
        }
        let mut bad = rows.clone();
        bad[3]["messages"][0]["details"]["rawEnvelope"] = json!(raw.trim());
        assert!(verify_observation(&bad, "native", raw, &sha).is_err());
        let mut bad = rows.clone();
        bad[2]["sessionId"] = json!("wrong");
        assert!(verify_observation(&bad, "native", raw, &sha).is_err());
        let mut bad = rows.clone();
        bad.swap(2, 3);
        assert!(verify_observation(&bad, "native", raw, &sha).is_err());
        let mut bad = rows.clone();
        bad.push(rows[2].clone());
        assert!(verify_observation(&bad, "native", raw, &sha).is_err());
        let mut bad = rows.clone();
        bad[3]["state"]["isStreaming"] = json!(true);
        assert!(verify_observation(&bad, "native", raw, &sha).is_err());
    }
    #[test]
    fn frozen_delivery_v1_identity_golden_vector() {
        let t = tempfile::tempdir().unwrap();
        let event =
            json!({"event_id":"e".repeat(64),"raw_payload":"{}","payload_sha256":digest(b"{}")});
        let bytes = serde_json::to_vec(&event).unwrap();
        let path = t.path().join("event.json");
        fs::write(&path, &bytes).unwrap();
        let binding = json!({"recipient":"target","principal":"fixture","job":"fixture","lane":"lane","session":"native","workspace":"/workspace","generation":1,"address":"role:target","fixture_mode":"observe","expires_at":4102444800u64,"active":true});
        let mut constraints = binding.clone();
        constraints.as_object_mut().unwrap().remove("session");
        let p = Profile {
            schema: PROFILE.into(),
            shim: Artifact {
                path: "/unused".into(),
                sha256: "0".repeat(64),
            },
            event: Artifact {
                path,
                sha256: digest(&bytes),
            },
            attempt_number: 1,
            recipient_without_session: constraints,
        };
        // Vector from exact accepted Binding Serde field order and frozen Store
        // tuple encodings: [event_id,recipient,generation,binding_sha], [delivery_id,1].
        let mut e = json!({"schema":"work-engine.delivery-envelope/v1","fixture_only":true,"authority":"observation_only","delivery_id":"c5527028f66098e2fd20da97955b9f8d504ddaff50124a8dd0ab12ac190c91f7","attempt_id":"7f69c2b7fd3a2e0feaf48fe0a715a4ebd038c6ea8286df364480c7bfc00f8188","subscription_id":"target","binding_sha256":"6db88a7532690180bec851e4dd1a2751a98dfac4c45db64b34913ca1dd37cd88","recipient":binding,"event":event});
        validate_envelope(&p, &e, "native").unwrap();
        e["attempt_id"] = json!("0".repeat(64));
        assert!(validate_envelope(&p, &e, "native").is_err());
    }
    #[test]
    fn readiness_and_complete_stream_fail_closed() {
        let rows = proof();
        assert_eq!(ready(&rows).unwrap(), Some("native".into()));
        let mut busy = rows.clone();
        busy[0]["state"]["isStreaming"] = json!(true);
        assert!(ready(&busy).is_err());
        let mut bytes = rows
            .iter()
            .map(|v| format!("{v}\n"))
            .collect::<String>()
            .into_bytes();
        assert!(frames(&bytes, true).is_ok());
        bytes.extend_from_slice(b"{\"partial\":");
        assert!(frames(&bytes, true).is_err());
        assert_eq!(frames(&bytes, false).unwrap().len(), rows.len());
    }
}
