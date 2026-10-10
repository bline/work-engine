//! Version-bound stdio protocol peer; no model, tools, or credentials.
use anyhow::{Result, bail};
use context_lifecycle_experiment::codex::{NativeStatus, Turn};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const THREAD: &str = "01999000-0000-7000-8000-000000000001";
const TURN: &str = "01999000-0000-7000-8000-000000000002";
#[derive(Default, Serialize, Deserialize)]
struct State {
    starts: u64,
    interrupts: u64,
    turn: Option<Turn>,
    start_params: Value,
    #[serde(default)]
    denied_requests: Vec<Value>,
}
fn save(home: &std::path::Path, state: &State) -> Result<()> {
    fs::write(home.join("fixture.next"), serde_json::to_vec(state)?)?;
    fs::rename(home.join("fixture.next"), home.join("fixture.json"))?;
    Ok(())
}
fn emit(output: &Arc<Mutex<std::io::Stdout>>, value: Value) -> Result<()> {
    let mut output = output.lock().expect("stdout mutex");
    serde_json::to_writer(&mut *output, &value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
fn notice(output: &Arc<Mutex<std::io::Stdout>>, turn: Turn, wrong: bool) -> Result<()> {
    emit(
        output,
        json!({"method":"turn/completed","params":{"threadId":if wrong {"wrong-thread"} else {THREAD},"turn":turn}}),
    )
}
fn thread_value(home: &std::path::Path, state: &State) -> Value {
    json!({"id":THREAD,"turns":state.turn.iter().collect::<Vec<_>>(),"ephemeral":false,
        "historyMode":"legacy","cliVersion":"0.160.1","createdAt":1,"updatedAt":1,
        "cwd":home.join("work"),"modelProvider":"local_probe","preview":"fixture",
        "projectId":null,"sessionId":THREAD,"source":"appServer","status":{"type":"idle"}})
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let scenario = args
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("scenario"))?
        .clone();
    let home = PathBuf::from(args.get(2).ok_or_else(|| anyhow::anyhow!("home"))?);
    let initial: State = fs::read(home.join("fixture.json"))
        .ok()
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?
        .unwrap_or_default();
    let state = Arc::new(Mutex::new(initial));
    let output = Arc::new(Mutex::new(std::io::stdout()));
    let listener = TcpListener::bind("127.0.0.1:0")?;
    fs::write(
        home.join("control.json"),
        serde_json::to_vec(&listener.local_addr()?)?,
    )?;
    let shared = state.clone();
    let writer = output.clone();
    let control_home = home.clone();
    std::thread::spawn(move || -> Result<()> {
        for stream in listener.incoming() {
            let mut stream = stream?;
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            stream.set_write_timeout(Some(Duration::from_secs(2)))?;
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line)?;
            let request: Value = serde_json::from_str(&line)?;
            let mut state = shared.lock().expect("state");
            let mut exit = false;
            match request["action"].as_str().unwrap_or("") {
                "snapshot" => {}
                "complete" | "conflict" | "silent-complete" => {
                    let turn = state.turn.as_mut().expect("started turn");
                    turn.status = if request["action"] == "conflict" {
                        NativeStatus::Failed
                    } else {
                        serde_json::from_value(request["status"].clone())?
                    };
                    if request["action"] != "silent-complete" {
                        notice(&writer, turn.clone(), false)?;
                    }
                }
                "duplicate" => notice(&writer, state.turn.clone().expect("turn"), false)?,
                "content-conflict" => {
                    for text in ["first finalized text", "contradictory finalized text"] {
                        let mut turn = state.turn.clone().expect("turn");
                        turn.status = NativeStatus::Completed;
                        turn.items =
                            vec![json!({"id":"same-final-item","type":"agentMessage","text":text})];
                        notice(&writer, turn, false)?;
                    }
                }
                "wrong" => {
                    let mut turn = state.turn.clone().expect("turn");
                    turn.status = NativeStatus::Completed;
                    notice(&writer, turn, true)?;
                }
                "tool" => emit(
                    &writer,
                    json!({"method":"item/started","params":{"threadId":THREAD,"turnId":TURN,"item":{"id":"uncertain-tool","type":"commandExecution","status":"inProgress"}}}),
                )?,
                "request-drain" => emit(
                    &writer,
                    json!({"id":"input-drain","method":"item/tool/requestUserInput","params":{"threadId":THREAD,"turnId":TURN,"questions":[]}}),
                )?,
                "malformed" => {
                    let mut out = writer.lock().expect("stdout");
                    out.write_all(b"not-json\n")?;
                    out.flush()?;
                }
                "exit" => exit = true,
                other => bail!("unknown fixture control {other}"),
            }
            save(&control_home, &state)?;
            serde_json::to_writer(&mut stream, &Ok::<_, String>(&*state))?;
            stream.write_all(b"\n")?;
            stream.flush()?;
            if exit {
                std::process::exit(0);
            }
        }
        Ok(())
    });
    for line in BufReader::new(std::io::stdin()).lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let Some(method) = request["method"].as_str() else {
            if request["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("input-"))
            {
                let mut state = state.lock().expect("state");
                state.denied_requests.push(request);
                save(&home, &state)?;
            }
            continue;
        };
        let id = request["id"].clone();
        let mut state = state.lock().expect("state");
        let result = match method {
            "initialize" => {
                json!({"userAgent":"codex-cli/0.160.1 fixture","codexHome":home.join("codex"),"platformFamily":"unix","platformOs":"linux"})
            }
            "initialized" => continue,
            "thread/start" => json!({"thread":thread_value(&home, &state)}),
            "turn/start" => {
                state.starts += 1;
                state.start_params = request["params"].clone();
                state.turn = Some(Turn {
                    id: TURN.into(),
                    status: NativeStatus::InProgress,
                    items: vec![],
                    error: None,
                });
                save(&home, &state)?;
                match scenario.as_str() {
                    "request-wait" => emit(
                        &output,
                        json!({"id":"input-wait","method":"item/tool/requestUserInput","params":{"threadId":THREAD,"turnId":TURN,"questions":[]}}),
                    )?,
                    "early" | "lost-response" => {
                        state.turn.as_mut().expect("turn").status = NativeStatus::Completed;
                        save(&home, &state)?;
                        notice(&output, state.turn.clone().expect("turn"), false)?;
                        if scenario == "lost-response" {
                            return Ok(());
                        }
                    }
                    "eof" => return Ok(()),
                    "malformed" => {
                        output.lock().expect("stdout").write_all(b"not-json\n")?;
                        return Ok(());
                    }
                    "inprogress-terminal" => {
                        notice(&output, state.turn.clone().expect("turn"), false)?
                    }
                    _ => {}
                }
                json!({"turn":state.turn})
            }
            "turn/interrupt" => {
                state.interrupts += 1;
                save(&home, &state)?;
                json!({})
            }
            "thread/read" => {
                if scenario == "full-empty"
                    && state
                        .turn
                        .as_ref()
                        .is_some_and(|turn| turn.status == NativeStatus::Completed)
                {
                    let mut thread = thread_value(&home, &state);
                    thread["turns"] = json!([{"id":TURN,"status":"completed","itemsView":"full","items":[{"type":"userMessage","id":"user-item","clientId":"effect","content":[{"type":"text","text":"Synthetic text one"}]}],"error":null}]);
                    json!({"thread":thread})
                } else {
                    json!({"thread":thread_value(&home, &state)})
                }
            }
            _ => {
                emit(
                    &output,
                    json!({"id":id,"error":{"code":-32601,"message":"unsupported fixture method"}}),
                )?;
                continue;
            }
        };
        emit(&output, json!({"id":id,"result":result}))?;
    }
    Ok(())
}
