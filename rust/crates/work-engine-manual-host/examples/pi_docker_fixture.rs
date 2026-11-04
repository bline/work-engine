//! Deliberately fake Docker daemon for isolated failure tests. Never live evidence.
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::Duration,
};
fn main() {
    let exe = std::env::current_exe().unwrap();
    let mode = fs::read_to_string(exe.with_extension("mode")).unwrap_or_default();
    let path = exe.with_extension("state");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verb = args.first().map(String::as_str).unwrap_or("");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(exe.with_extension("calls"))
        .unwrap();
    writeln!(log, "{verb}").unwrap();
    let get = |key: &str| {
        args.windows(2)
            .find(|a| a[0] == key)
            .map(|a| a[1].clone())
            .unwrap_or_default()
    };
    let load = || -> Value { serde_json::from_slice(&fs::read(&path).unwrap()).unwrap() };
    let save = |value: &Value| {
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    };
    match verb {
        "create" => {
            let labels: serde_json::Map<String, Value> = args
                .windows(2)
                .filter(|a| a[0] == "--label")
                .map(|a| {
                    let (k, v) = a[1].split_once('=').unwrap();
                    (k.into(), json!(v))
                })
                .collect();
            let mounts: Vec<Value> = args.windows(2).filter(|a| a[0]=="--mount").map(|a| {
                let parts: Vec<_> = a[1].split(',').collect();
                let source=parts.iter().find_map(|p| p.strip_prefix("src=")).unwrap();
                let target=parts.iter().find_map(|p| p.strip_prefix("dst=")).unwrap();
                json!({"Type":"bind","Source":source,"Destination":target,"RW":!parts.contains(&"readonly")})
            }).collect();
            let image = args.iter().position(|a| a.contains("@sha256:")).unwrap();
            let mut value = json!({"Id":"a".repeat(64),"Name":format!("/{}",get("--name")),"Config":{"Labels":labels,"Image":args[image],"User":"1000:1000","Cmd":args[image+1..],"Entrypoint":["docker-entrypoint.sh"],"WorkingDir":"/workspace"},"HostConfig":{"NetworkMode":"none","ReadonlyRootfs":true,"Privileged":false,"CapDrop":["ALL"],"SecurityOpt":["no-new-privileges"],"Memory":536870912_u64,"PidsLimit":64,"NanoCpus":1000000000_u64,"Tmpfs":{"/tmp":"rw,noexec,nosuid,size=64m"}},"Mounts":mounts,"State":{"Status":"created","Running":false,"ExitCode":0,"OOMKilled":false}});
            if mode == "bad-mount" {
                value["Mounts"][0]["RW"] = json!(true);
            }
            save(&value);
            if mode == "create-uncertain" {
                std::process::exit(1);
            }
            println!("{}", "a".repeat(64));
        }
        "inspect" => {
            if mode == "inspect-error" || !path.exists() {
                std::process::exit(1);
            }
            println!("{}", json!([load()]));
        }
        "start" => {
            let mut value = load();
            value["State"]["Status"] = json!("running");
            value["State"]["Running"] = json!(true);
            save(&value);
            if mode == "start-stall" {
                loop {
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
            for line in io::stdin().lock().lines() {
                let command: Value = serde_json::from_str(&line.unwrap()).unwrap();
                let mut response = json!({"id":command["id"],"type":"response","command":command["type"],"success":true});
                if command["type"] == "get_state" {
                    response["data"] = json!({"sessionId":"fixture-session","model":{"provider":"unknown"},"autoCompactionEnabled":command["id"]=="state-before","isStreaming":false,"isCompacting":false,"messageCount":0,"pendingMessageCount":0});
                }
                if mode == "rpc-mismatch" {
                    response["id"] = json!("foreign-session");
                }
                if mode == "rpc-truncated" {
                    print!("{{\"type\":");
                    io::stdout().flush().unwrap();
                    break;
                }
                println!("{response}");
                io::stdout().flush().unwrap();
            }
            value["State"]["Status"] = json!("exited");
            value["State"]["Running"] = json!(false);
            save(&value);
            let exports = value["Mounts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["Destination"] == "/export")
                .unwrap()["Source"]
                .as_str()
                .unwrap();
            fs::write(
                Path::new(exports).join("fixture-output.txt"),
                "fake daemon artifact\n",
            )
            .unwrap();
        }
        "stop" | "kill" => {
            let mut value = load();
            value["State"]["Status"] = json!("exited");
            value["State"]["Running"] = json!(false);
            value["State"]["ExitCode"] = json!(143);
            save(&value);
            println!("stopped");
        }
        "wait" => {
            if mode == "wait-mismatch" {
                println!("9");
            } else {
                println!("{}", load()["State"]["ExitCode"]);
            }
        }
        "rm" => {
            if mode == "remove-error" {
                std::process::exit(1);
            }
            fs::remove_file(path).unwrap();
            println!("removed");
        }
        "ps" => {
            if path.exists() {
                println!("{}", "a".repeat(64));
            }
        }
        _ => std::process::exit(2),
    }
}
