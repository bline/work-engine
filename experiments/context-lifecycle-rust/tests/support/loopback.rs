//! Bounded local Responses simulator. Never forwards traffic or accepts credentials.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Captured {
    pub authorization: bool,
    pub body: Value,
    pub response_events: Vec<Value>,
}
pub struct Backend {
    pub address: SocketAddr,
    requests: Arc<Mutex<Vec<Captured>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<()>>>,
}
#[derive(Clone, Copy)]
enum Probe {
    Legacy,
    PureDiscovery,
    UserInput,
    AsyncUserInput,
    TextVertical,
}
impl Backend {
    pub fn start() -> Result<Self> {
        Self::start_with(Probe::Legacy)
    }
    pub fn start_pure_discovery() -> Result<Self> {
        Self::start_with(Probe::PureDiscovery)
    }
    pub fn start_user_input() -> Result<Self> {
        Self::start_with(Probe::UserInput)
    }
    pub fn start_async_user_input() -> Result<Self> {
        Self::start_with(Probe::AsyncUserInput)
    }
    pub fn start_text_vertical() -> Result<Self> {
        Self::start_with(Probe::TextVertical)
    }
    fn start_with(probe: Probe) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let requests = Arc::new(Mutex::new(vec![]));
        let stop = Arc::new(AtomicBool::new(false));
        let captured = requests.clone();
        let stopping = stop.clone();
        let worker = std::thread::spawn(move || -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(30);
            while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                        let request = read_request(&stream)?;
                        ensure!(!request.authorization, "authentication header forbidden");
                        let count = {
                            let mut all = captured.lock().expect("requests");
                            all.push(request);
                            all.len()
                        };
                        ensure!(
                            count
                                <= match probe {
                                    Probe::Legacy => 4,
                                    Probe::PureDiscovery
                                    | Probe::UserInput
                                    | Probe::AsyncUserInput => 2,
                                    Probe::TextVertical => 3,
                                },
                            "unexpected extra native inference request"
                        );
                        let item = if matches!(probe, Probe::TextVertical) {
                            let user_texts: Vec<_> = captured.lock().expect("requests")[count - 1]
                                .body["input"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter(|item| item["role"] == "user")
                                .flat_map(|item| item["content"].as_array().into_iter().flatten())
                                .filter_map(|content| content["text"].as_str().map(str::to_owned))
                                .collect();
                            let output = match count {
                                1 => {
                                    ensure!(
                                        user_texts.iter().any(|text| text == "Synthetic text one"),
                                        "first native input mismatch"
                                    );
                                    "FACT:ORION-7".to_owned()
                                }
                                2 => {
                                    let package_text = user_texts.last().ok_or_else(|| {
                                        anyhow::anyhow!("readiness user input missing")
                                    })?;
                                    let package: Value = serde_json::from_str(package_text)?;
                                    ensure!(
                                        package["kind"] == "native-readiness"
                                            && package["package"]["continuation"]["references"][0]
                                                == "FACT:ORION-7",
                                        "readiness continuation fact mismatch"
                                    );
                                    package["expected_ack"]
                                        .as_str()
                                        .ok_or_else(|| anyhow::anyhow!("readiness ack missing"))?
                                        .to_owned()
                                }
                                _ => {
                                    ensure!(
                                        user_texts
                                            .last()
                                            .is_some_and(|text| text.contains("SECOND:")
                                                && !text.contains("ORION-7"))
                                            && user_texts[..user_texts.len() - 1]
                                                .iter()
                                                .any(|text| text.contains("FACT:ORION-7")),
                                        "successor input/context fact mismatch"
                                    );
                                    "SECOND:ORION-7".to_owned()
                                }
                            };
                            json!({"type":"message","id":format!("vertical-message-{count}"),"role":"assistant","status":"completed","content":[{"type":"output_text","text":output,"annotations":[]}]})
                        } else if matches!(probe, Probe::PureDiscovery) && count == 1 {
                            json!({"type":"custom_tool_call","id":"pure-discovery-item","call_id":"pure-discovery-call","namespace":"functions","name":"exec","input":"text({declaredTools:ALL_TOOLS.map(({name})=>name),callableToolNames:Object.keys(tools),globals:Object.getOwnPropertyNames(globalThis),ambientAccess:{process:typeof process,require:typeof require,fetch:typeof fetch,WebSocket:typeof WebSocket,Deno:typeof Deno,Bun:typeof Bun}});"})
                        } else if matches!(probe, Probe::PureDiscovery) {
                            json!({"type":"message","id":"pure-discovery-final","role":"assistant","status":"completed","content":[{"type":"output_text","text":"pure discovery complete","annotations":[]}]})
                        } else if matches!(probe, Probe::UserInput) && count == 1 {
                            json!({"type":"function_call","id":"user-input-item","call_id":"user-input-call","namespace":"functions","name":"request_user_input","arguments":"{\"questions\":[{\"id\":\"probe\",\"header\":\"Probe\",\"question\":\"Select one\",\"options\":[{\"label\":\"Continue (Recommended)\",\"description\":\"Local capability probe\"},{\"label\":\"Stop\",\"description\":\"Local capability probe\"}]}]}"})
                        } else if matches!(probe, Probe::UserInput) {
                            json!({"type":"message","id":"user-input-final","role":"assistant","status":"completed","content":[{"type":"output_text","text":"user input probe complete","annotations":[]}]})
                        } else if matches!(probe, Probe::AsyncUserInput) && count == 1 {
                            json!({"type":"function_call","id":"async-user-input-item","call_id":"async-user-input-call","namespace":"functions","name":"request_user_input_async","arguments":"{\"questions\":[{\"title\":\"Local capability probe\",\"options\":[\"Continue\",\"Stop\"]}]}"})
                        } else if matches!(probe, Probe::AsyncUserInput) {
                            json!({"type":"message","id":"async-user-input-final","role":"assistant","status":"completed","content":[{"type":"output_text","text":"async user input probe complete","annotations":[]}]})
                        } else {
                            match count {
                                1 => {
                                    json!({"type":"function_call","id":"function-item-1","call_id":"disabled-shell-probe","name":"exec_command","arguments":"{\"cmd\":\"printf DISABLED_TOOL_EXECUTED\"}"})
                                }
                                2 => {
                                    json!({"type":"function_call","id":"function-item-2","call_id":"disabled-create-goal-probe","name":"create_goal","arguments":"{\"objective\":\"probe must never persist\"}"})
                                }
                                3 => {
                                    json!({"type":"function_call","id":"function-item-3","call_id":"disabled-update-goal-probe","name":"update_goal","arguments":"{\"status\":\"complete\"}"})
                                }
                                _ => {
                                    json!({"type":"message","id":"message-item","role":"assistant","status":"completed","content":[{"type":"output_text","text":"loopback contract probe complete","annotations":[]}]})
                                }
                            }
                        };
                        let events = if matches!(probe, Probe::TextVertical) {
                            let output =
                                item["content"][0]["text"].as_str().expect("vertical text");
                            let (left, right) = output.split_at(output.len() / 2);
                            let message_id = item["id"].clone();
                            let reasoning_id = format!("vertical-reasoning-{count}");
                            let reasoning_text = "Briefly identify the carried continuation fact.";
                            let reasoning = json!({"type":"reasoning","id":reasoning_id,"status":"completed","summary":[{"type":"summary_text","text":reasoning_text}]});
                            let response = json!({"id":format!("response-{count}"),"object":"response","created_at":1,"status":"completed","output":[reasoning.clone(),item.clone()],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}});
                            vec![
                                json!({"type":"response.created","response":{"id":format!("response-{count}"),"status":"in_progress","output":[]}}),
                                json!({"type":"response.output_item.added","output_index":0,"item":{"type":"reasoning","id":reasoning_id,"status":"in_progress","summary":[]}}),
                                json!({"type":"response.reasoning_summary_part.added","item_id":reasoning_id,"output_index":0,"summary_index":0,"part":{"type":"summary_text","text":""}}),
                                json!({"type":"response.reasoning_summary_text.delta","item_id":reasoning_id,"output_index":0,"summary_index":0,"delta":reasoning_text}),
                                json!({"type":"response.output_item.done","output_index":0,"item":reasoning}),
                                json!({"type":"response.output_item.added","output_index":1,"item":{"type":"message","id":message_id,"role":"assistant","status":"in_progress","content":[]}}),
                                json!({"type":"response.content_part.added","item_id":message_id,"output_index":1,"content_index":0,"part":{"type":"output_text","text":"","annotations":[]}}),
                                json!({"type":"response.output_text.delta","item_id":message_id,"output_index":1,"content_index":0,"delta":left}),
                                json!({"type":"response.output_text.delta","item_id":message_id,"output_index":1,"content_index":0,"delta":right}),
                                json!({"type":"response.output_item.done","output_index":1,"item":item}),
                                json!({"type":"response.completed","response":response}),
                            ]
                        } else {
                            let response = json!({"id":format!("response-{count}"),"object":"response","created_at":1,"status":"completed","output":[item.clone()],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}});
                            vec![
                                json!({"type":"response.created","response":{"id":format!("response-{count}"),"status":"in_progress","output":[]}}),
                                json!({"type":"response.output_item.done","output_index":0,"item":item}),
                                json!({"type":"response.completed","response":response}),
                            ]
                        };
                        captured.lock().expect("requests")[count - 1].response_events =
                            events.to_vec();
                        let body = events
                            .iter()
                            .map(|event| format!("data: {event}\n\n"))
                            .collect::<String>();
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )?;
                        stream.flush()?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(())
        });
        Ok(Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        })
    }
    pub fn requests(&self) -> Result<Vec<Captured>> {
        Ok(self
            .requests
            .lock()
            .map_err(|_| anyhow::anyhow!("capture poisoned"))?
            .clone())
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn read_request(stream: &TcpStream) -> Result<Captured> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    ensure!(
        line.starts_with("POST /v1/responses "),
        "unexpected local HTTP route"
    );
    let mut length = 0;
    let mut authorization = false;
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        ensure!(!line.is_empty(), "HTTP header EOF");
        if line == "\r\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = value.trim().parse()?;
        }
        if lower.starts_with("authorization:") {
            authorization = true;
        }
        ensure!(
            !lower.starts_with("content-encoding:"),
            "compressed request is outside the bounded probe"
        );
    }
    ensure!(
        (1..=2_000_000).contains(&length),
        "bounded content length required"
    );
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Captured {
        authorization,
        body: serde_json::from_slice(&body)?,
        response_events: vec![],
    })
}
