use serde_json::{Value, json};
use std::{
    collections::HashSet,
    os::unix::net::UnixStream,
    path::Path,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, client::client_with_config, protocol::WebSocketConfig};

const MAX_HISTORY: usize = 10_000;
const MAX_PAGES: usize = 100;
const MAX_MESSAGE: usize = 2 * 1024 * 1024;

pub struct Native {
    ws: WebSocket<UnixStream>,
    id: u64,
}
impl Native {
    pub fn connect(path: &str) -> Result<Self, ()> {
        let stream = UnixStream::connect(Path::new(path)).map_err(|_| ())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| ())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| ())?;
        let (ws, _) = client_with_config(
            "ws://localhost/",
            stream,
            Some(
                WebSocketConfig::default()
                    .max_message_size(Some(MAX_MESSAGE))
                    .max_frame_size(Some(MAX_MESSAGE)),
            ),
        )
        .map_err(|_| ())?;
        let mut me = Self { ws, id: 0 };
        me.call(
            "initialize",
            json!({"clientInfo":{"name":"session-bridge","version":"1"}}),
        )?;
        me.ws
            .send(Message::Text(
                json!({"method":"initialized"}).to_string().into(),
            ))
            .map_err(|_| ())?;
        Ok(me)
    }
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, ()> {
        self.id += 1;
        let id = self.id;
        self.ws
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .map_err(|_| ())?;
        let deadline = Instant::now() + Duration::from_secs(10);
        for _ in 0..1024 {
            let remaining = deadline.checked_duration_since(Instant::now()).ok_or(())?;
            self.ws
                .get_mut()
                .set_read_timeout(Some(remaining))
                .map_err(|_| ())?;
            let message = self.ws.read().map_err(|_| ())?;
            if message.len() > MAX_MESSAGE {
                return Err(());
            }
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).map_err(|_| ())?;
                if value.get("id").and_then(Value::as_u64) == Some(id) {
                    if value.get("error").is_some() {
                        return Err(());
                    }
                    return value.get("result").cloned().ok_or(());
                }
            }
        }
        Err(())
    }
    fn pages(&mut self, method: &str, session: Option<&str>) -> Result<Vec<Value>, ()> {
        let mut cursor: Option<String> = None;
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let mut bytes = 0usize;
        for _ in 0..MAX_PAGES {
            let mut p = json!({"limit":100});
            if let Some(s) = session {
                p["threadId"] = json!(s);
                p["sortDirection"] = json!("asc");
                p["itemsView"] = json!("full");
            }
            if let Some(c) = &cursor {
                p["cursor"] = json!(c);
            }
            let page = self.call(method, p)?;
            bytes = bytes
                .checked_add(serde_json::to_vec(&page).map_err(|_| ())?.len())
                .ok_or(())?;
            if bytes > 8 * 1024 * 1024 {
                return Err(());
            }
            let data = page.get("data").and_then(Value::as_array).ok_or(())?;
            result.extend(data.iter().cloned());
            if result.len() > MAX_HISTORY {
                return Err(());
            }
            match page.get("nextCursor") {
                Some(Value::Null) => return Ok(result),
                Some(Value::String(c)) if !c.is_empty() && seen.insert(c.clone()) => {
                    cursor = Some(c.clone())
                }
                _ => return Err(()),
            }
        }
        Err(())
    }
    pub fn history(&mut self, session: &str) -> Result<Vec<Value>, ()> {
        let turns = self.pages("thread/turns/list", Some(session))?;
        let mut ids = HashSet::new();
        for turn in &turns {
            let id = turn.get("id").and_then(Value::as_str).ok_or(())?;
            let status = turn.get("status").and_then(Value::as_str).ok_or(())?;
            if !crate::wire::valid_uuid(id)
                || !ids.insert(id.to_owned())
                || turn.get("items").and_then(Value::as_array).is_none()
                || !["inProgress", "completed", "failed", "interrupted"].contains(&status)
            {
                return Err(());
            }
        }
        Ok(turns)
    }
    pub fn verify_target(&mut self, session: &str, workspace: &str) -> Result<(), ()> {
        let loaded = self.pages("thread/loaded/list", None)?;
        if loaded
            .iter()
            .filter(|v| v.as_str() == Some(session))
            .count()
            != 1
        {
            return Err(());
        }
        let response = self.call("thread/read", json!({"threadId":session}))?;
        let t = response.get("thread").ok_or(())?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
        {
            return Err(());
        }
        Ok(())
    }
    pub fn preflight(
        &mut self,
        session: &str,
        workspace: &str,
    ) -> Result<Vec<Value>, &'static str> {
        let unavailable = |_| "history_unavailable";
        let loaded = self
            .pages("thread/loaded/list", None)
            .map_err(unavailable)?;
        if loaded
            .iter()
            .filter(|v| v.as_str() == Some(session))
            .count()
            != 1
        {
            return Err("history_unavailable");
        }
        let response = self
            .call("thread/read", json!({"threadId":session}))
            .map_err(unavailable)?;
        let t = response.get("thread").ok_or("history_unavailable")?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
        {
            return Err("target_mismatch");
        }
        match t.pointer("/status/type").and_then(Value::as_str) {
            Some("idle") => (),
            Some("active") => return Err("session_busy"),
            _ => return Err("history_unavailable"),
        }
        let first = self.history(session).map_err(unavailable)?;
        if first
            .iter()
            .any(|t| t.get("status").and_then(Value::as_str) == Some("inProgress"))
        {
            return Err("session_busy");
        }
        if first != self.history(session).map_err(unavailable)? {
            return Err("history_unavailable");
        }
        Ok(first)
    }
    pub fn resume(&mut self, session: &str, workspace: &str) -> Result<(), ()> {
        let response = self.call("thread/resume", json!({"threadId":session}))?;
        let t = response.get("thread").ok_or(())?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
            || t.pointer("/status/type").and_then(Value::as_str) != Some("idle")
        {
            return Err(());
        }
        Ok(())
    }
    pub fn start(&mut self, session: &str, operation: &str, wire: &str) -> Result<String, ()> {
        let r = self.call("turn/start", json!({"threadId":session,"input":[{"type":"text","text":wire}],"clientUserMessageId":operation}))?;
        r.pointer("/turn/id")
            .and_then(Value::as_str)
            .filter(|s| crate::wire::valid_uuid(s))
            .map(str::to_owned)
            .ok_or(())
    }
}
