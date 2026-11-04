use serde_json::{Value, json};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use workspace_publication::RecordRef;

fn reference(path: &str) -> Result<RecordRef, String> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        return Err("usage: work-engine-completionctl SOCKET show-package|show-offer|accept-checkpoint|create|decline|cancel|prepare-publication|open-offer|inspect-operation|reconcile|close-no-effect ...".into());
    }
    let socket = &a[1];
    let command = &a[2];
    let request: Value = match command.as_str() {
        "inspect-candidate" if a.len() == 6 => {
            let identity: Value =
                serde_json::from_slice(&std::fs::read(&a[3]).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let locator: Value =
                serde_json::from_slice(&std::fs::read(&a[4]).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let manifest: Value =
                serde_json::from_slice(&std::fs::read(&a[5]).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            json!({"command":"inspect_candidate","identity":identity,"locator":locator,"manifest":manifest})
        }
        "show-package" if a.len() == 4 => {
            json!({"command":"show_package","packageRef":reference(&a[3])?})
        }
        "show-offer" if a.len() == 4 => {
            json!({"command":"show_offer","offerRef":reference(&a[3])?})
        }
        "accept-checkpoint" if a.len() == 6 => {
            json!({"command":"accept_checkpoint","packageRef":reference(&a[3])?,"displayedDigest":a[4],"requestId":a[5]})
        }
        "prepare-publication" if a.len() == 7 => {
            json!({"command":"prepare_publication","eligibilityRef":reference(&a[3])?,"targetRef":a[4],"message":std::fs::read_to_string(&a[5]).map_err(|e|e.to_string())?,"requestId":a[6]})
        }
        "open-offer" if a.len() == 4 => {
            json!({"command":"open_offer","preparedRef":reference(&a[3])?})
        }
        "create" | "decline" | "cancel" if a.len() == 6 => {
            json!({"command":command,"offerRef":reference(&a[3])?,"displayedDigest":a[4],"requestId":a[5]})
        }
        "revoke" if a.len() == 5 => {
            json!({"command":"revoke","eligibilityRef":reference(&a[3])?,"requestId":a[4]})
        }
        "inspect-operation" if a.len() == 4 => {
            json!({"command":"inspect_operation","operationId":a[3]})
        }
        "reconcile" | "close-no-effect" if a.len() == 4 => {
            let req: Value =
                serde_json::from_slice(&std::fs::read(&a[3]).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            json!({"command":if command=="reconcile"{"reconcile"}else{"close_no_effect"},"publicationRequest":req})
        }
        _ => return Err("wrong command arguments".into()),
    };
    let mut stream = UnixStream::connect(socket).map_err(|e| e.to_string())?;
    stream
        .write_all(&serde_json::to_vec(&request).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let result: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    if result["ok"] != true {
        return Err(result["error"]
            .as_str()
            .unwrap_or("completion host rejected command")
            .into());
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
