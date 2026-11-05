mod claims;
mod command;
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use session_query::{INPUT_LIMIT, Profile, Query};
use sha2::{Digest, Sha256};
use std::{
    env,
    io::Read,
    net::{IpAddr, SocketAddr},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, unix::AsyncFd},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, watch},
    task::JoinSet,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u8,
    listen: SocketAddr,
    mcp: bool,
    query_profile: String,
    credential_file: String,
    #[serde(default)]
    command_profile: Option<String>,
    #[serde(default)]
    command_capability: Option<String>,
    #[serde(default)]
    repository_claims_profile: Option<String>,
    max_connections: usize,
    max_queries: usize,
    request_timeout_seconds: u64,
}
struct App {
    repository_claims: Option<claims::Reads>,
    _claims_profile_lock: Option<std::fs::File>,
    profile: Profile,
    credential: Vec<u8>,
    command_profile: Option<Profile>,
    _command_profile_lock: Option<std::fs::File>,
    identity: String,
    build: String,
    slots: Arc<Semaphore>,
    _lock: std::fs::File,
    _profile_lock: std::fs::File,
}

// Reject duplicate keys at every nesting depth before interpreting any request.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element::<Unique>()? {
                    out.push(v.0);
                }
                Ok(Unique(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, v)) = a.next_entry::<String, Unique>()? {
                    if out.insert(k, v.0).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(Unique(Value::Object(out)))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u8,
    operation: String,
    arguments: Value,
    request_id: String,
}
impl App {
    async fn call(self: &Arc<Self>, r: Request) -> Value {
        if r.schema_version != 1 || r.request_id.is_empty() || r.request_id.len() > 128 {
            return json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"error":{"code":"invalid_request"}});
        }
        if claims::Reads::operation(&r.operation) {
            let app = self.clone();
            let permit = match self.slots.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => {
                    return json!({"schema_version":1,"request_id":r.request_id,"error":{"code":"owner_unavailable"}});
                }
            };
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                app.repository_claims
                    .as_ref()
                    .ok_or("claim_scope_unavailable")?
                    .call(&r.operation, r.arguments)
            })
            .await;
            return match result {
                Ok(Ok(value)) if value.to_string().len() <= 1024 * 1024 => {
                    json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_result":value})
                }
                Ok(Ok(_)) => {
                    json!({"schema_version":1,"request_id":r.request_id,"owner_error":{"code":"output_too_large"}})
                }
                Ok(Err(code)) => {
                    json!({"schema_version":1,"request_id":r.request_id,"owner_error":{"code":code}})
                }
                Err(_) => {
                    json!({"schema_version":1,"request_id":r.request_id,"owner_error":{"code":"claim_owner_unavailable"}})
                }
            };
        }
        let is_command = matches!(
            r.operation.as_str(),
            "submit_session_request" | "read_session_operation" | "read_session_result"
        );
        if is_command && self.command_profile.is_none() {
            return json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"error":{"code":"invalid_request"}});
        }
        let submit = r.operation == "submit_session_request";
        let q = if submit {
            None
        } else {
            match Query::parse(&r.operation, r.arguments.clone()) {
                Ok(q) => Some(q),
                Err(e) => {
                    return json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"error":{"code":e}});
                }
            }
        };
        let permit = match self.slots.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => {
                return json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_error":{"code":"owner_unavailable"}});
            }
        };
        let app = self.clone();
        let res = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let profile = if is_command {
                app.command_profile.as_ref().unwrap()
            } else {
                &app.profile
            };
            if submit {
                command::submit(profile, r.arguments)
            } else {
                session_query::query(profile, q.as_ref().unwrap())
            }
        })
        .await;
        match res {
            Ok(Ok(v)) => {
                let result = json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_result":v});
                if result.to_string().len() > 1024 * 1024 + 8192 {
                    json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_error":{"code":"output_too_large"}})
                } else {
                    result
                }
            }
            Ok(Err(e)) => {
                json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_error":{"code":e}})
            }
            Err(_) => {
                json!({"schema_version":1,"request_id":r.request_id,"server_identity":self.identity,"server_build":self.build,"owner_error":{"code":"owner_unavailable"}})
            }
        }
    }
}
fn lock_private(path: &str) -> Result<(std::fs::File, Vec<u8>), &'static str> {
    let mut lock = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "invalid_config")?;
    let meta = lock.metadata().map_err(|_| "invalid_config")?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
    {
        return Err("invalid_config");
    }
    if meta.len() > INPUT_LIMIT as u64 {
        return Err("invalid_config");
    }
    let mut bytes = Vec::new();
    (&mut lock)
        .take(INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "invalid_config")?;
    if bytes.len() > INPUT_LIMIT {
        return Err("invalid_config");
    }
    Ok((lock, bytes))
}
fn load(path: &str) -> Result<(Config, Arc<App>), &'static str> {
    let (lock, bytes) = lock_private(path)?;
    let config: Config = serde_json::from_slice(&bytes).map_err(|_| "invalid_config")?;
    if !std::path::Path::new(path).is_absolute()
        || !std::path::Path::new(&config.query_profile).is_absolute()
        || !std::path::Path::new(&config.credential_file).is_absolute()
        || config.version != 1
        || !matches!(config.listen.ip(), IpAddr::V4(ip) if ip.is_loopback())
            && !matches!(config.listen.ip(), IpAddr::V6(ip) if ip.is_loopback())
        || config.listen.port() == 0
        || config.max_connections == 0
        || config.max_connections > 128
        || config.max_queries == 0
        || config.max_queries > 32
        || !(1..=120).contains(&config.request_timeout_seconds)
    {
        return Err("invalid_config");
    }
    let (profile_lock, p) = lock_private(&config.query_profile).map_err(|_| "invalid_profile")?;
    let profile: Profile = serde_json::from_slice(&p).map_err(|_| "invalid_profile")?;
    profile.validate()?;
    if profile.bridge.is_some() {
        return Err("invalid_profile");
    }
    if !matches!(
        (&config.command_capability, &config.command_profile),
        (None, None) | (Some(_), Some(_))
    ) || config
        .command_capability
        .as_deref()
        .is_some_and(|c| c != "offline-synthetic-v1")
    {
        return Err("invalid_profile");
    }
    let (command_profile_lock, command_profile, command_bytes) =
        if let Some(path) = &config.command_profile {
            if !std::path::Path::new(path).is_absolute() {
                return Err("invalid_profile");
            }
            let (lock, bytes) = lock_private(path).map_err(|_| "invalid_profile")?;
            let profile: Profile = serde_json::from_slice(&bytes).map_err(|_| "invalid_profile")?;
            profile.validate()?;
            if profile.bridge.is_none() {
                return Err("invalid_profile");
            }
            (Some(lock), Some(profile), bytes)
        } else {
            (None, None, Vec::new())
        };
    let (claims_profile_lock, repository_claims, claims_bytes) =
        if let Some(path) = &config.repository_claims_profile {
            if !std::path::Path::new(path).is_absolute() {
                return Err("invalid_claim_profile");
            }
            let (lock, bytes) = lock_private(path).map_err(|_| "invalid_claim_profile")?;
            let profile = serde_json::from_slice::<claims::ReadProfile>(&bytes)
                .map_err(|_| "invalid_claim_profile")?;
            (Some(lock), Some(claims::Reads::load(profile)?), bytes)
        } else {
            (None, None, Vec::new())
        };
    let credential = session_query::read_file(&config.credential_file, 4096, true)?;
    if credential.len() < 32
        || credential.len() > 4096
        || credential.iter().any(|b| !b.is_ascii_graphic())
    {
        return Err("invalid_credential");
    }
    let mut hash = Sha256::new();
    hash.update(&bytes);
    hash.update(&p);
    hash.update(&command_bytes);
    hash.update(&claims_bytes);
    let identity = format!("sha256:{:x}", hash.finalize());
    let executable = std::env::current_exe().map_err(|_| "invalid_build")?;
    let mut f = std::fs::File::open(executable).map_err(|_| "invalid_build")?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut f, &mut hasher).map_err(|_| "invalid_build")?;
    let build = format!("sha256:{:x}", hasher.finalize());
    let app = Arc::new(App {
        repository_claims,
        _claims_profile_lock: claims_profile_lock,
        profile,
        command_profile,
        _command_profile_lock: command_profile_lock,
        credential,
        identity,
        build,
        slots: Arc::new(Semaphore::new(config.max_queries)),
        _lock: lock,
        _profile_lock: profile_lock,
    });
    Ok((config, app))
}
async fn http(mut stream: TcpStream, addr: SocketAddr, app: Arc<App>, timeout: Duration) {
    let result = tokio::time::timeout(timeout, async {
        let mut buf = Vec::new();
        let mut part = [0u8; 4096];
        let split = loop {
            let n = stream.read(&mut part).await.map_err(|_| ())?;
            if n == 0 {
                return Err(());
            }
            buf.extend_from_slice(&part[..n]);
            if buf.len() > 16384 {
                return Err(());
            }
            if let Some(i) = buf.windows(4).position(|x| x == b"\r\n\r\n") {
                break i + 4;
            }
        };
        let header = std::str::from_utf8(&buf[..split]).map_err(|_| ())?;
        let mut lines = header.split("\r\n");
        let request = lines.next().ok_or(())?;
        let mut host = None;
        let mut auth = None;
        let mut typ = None;
        let mut len = None;
        let mut origin = false;
        for line in lines.filter(|s| !s.is_empty()) {
            let (k, v) = line.split_once(':').ok_or(())?;
            let v = v.trim();
            match k.to_ascii_lowercase().as_str() {
                "host" if host.is_none() => host = Some(v.to_string()),
                "authorization" if auth.is_none() => auth = Some(v.to_string()),
                "content-type" if typ.is_none() => typ = Some(v.to_string()),
                "content-length" if len.is_none() => {
                    len = Some(v.parse::<usize>().map_err(|_| ())?)
                }
                "origin" => origin = true,
                "transfer-encoding" => return Err(()),
                "host" | "authorization" | "content-type" | "content-length" => return Err(()),
                _ => {}
            }
        }
        let expected = if addr.is_ipv6() {
            format!("[{}]:{}", addr.ip(), addr.port())
        } else {
            format!("{}:{}", addr.ip(), addr.port())
        };
        if host.as_deref() != Some(&expected) || origin {
            return Err(());
        }
        if request == "GET /live HTTP/1.1" {
            return Ok((200, json!({"live":true})));
        }
        let supplied = auth
            .as_deref()
            .and_then(|x| x.strip_prefix("Bearer "))
            .unwrap_or("")
            .as_bytes();
        let mut diff = supplied.len() ^ app.credential.len();
        for (i, b) in app.credential.iter().enumerate() {
            diff |= usize::from(*b ^ supplied.get(i).copied().unwrap_or(0));
        }
        if diff != 0 {
            return Ok((401, json!({"error":"unauthorized"})));
        }
        if request == "GET /ready HTTP/1.1" {
            let owner = session_query::read_file(&app.profile.owner, 512 * 1024, false);
            let pinned = owner
                .is_ok_and(|b| format!("{:x}", Sha256::digest(b)) == app.profile.owner_sha256)
                && std::path::Path::new("/usr/bin/python3").is_file();
            let ready = if pinned {
                app.call(Request {
                    schema_version: 1,
                    operation: "list_sessions".into(),
                    arguments: json!({}),
                    request_id: "readiness".into(),
                })
                .await
                .get("owner_result")
                .is_some()
            } else {
                false
            };
            return Ok((200, json!({"list_sessions":ready,"read_session":ready})));
        }
        if request != "POST /v1/session-queries HTTP/1.1"
            || typ.as_deref() != Some("application/json")
        {
            return Err(());
        }
        let n = len.ok_or(())?;
        if n > INPUT_LIMIT || buf.len() - split > n {
            return Err(());
        }
        while buf.len() - split < n {
            let count = stream.read(&mut part).await.map_err(|_| ())?;
            if count == 0 {
                return Err(());
            }
            buf.extend_from_slice(&part[..count]);
            if buf.len() - split > n {
                return Err(());
            }
        }
        let unique: Unique = serde_json::from_slice(&buf[split..]).map_err(|_| ())?;
        let r: Request = serde_json::from_value(unique.0).map_err(|_| ())?;
        Ok((200, app.call(r).await))
    })
    .await;
    let (status, body) = match result {
        Ok(Ok(v)) => v,
        Ok(Err(())) => (400, json!({"error":"invalid_transport"})),
        Err(_) => (408, json!({"error":"deadline"})),
    };
    let mut data = body.to_string();
    if data.len() > 1024 * 1024 + 16384 {
        data = json!({"error":"output_too_large"}).to_string();
    }
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        if status == 200 { "OK" } else { "Error" },
        data.len(),
        data
    );
    let _ = tokio::time::timeout(timeout, stream.write_all(response.as_bytes())).await;
}
async fn mcp_write(output: &AsyncFd<std::fs::File>, mut bytes: &[u8]) -> Result<(), ()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !bytes.is_empty() {
            let mut guard = output.writable().await.map_err(|_| ())?;
            match guard.try_io(|inner| {
                let mut f = inner.get_ref();
                std::io::Write::write(&mut f, bytes)
            }) {
                Ok(Ok(0)) | Ok(Err(_)) => return Err(()),
                Ok(Ok(n)) => bytes = &bytes[n..],
                Err(_) => continue,
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| ())?
}
async fn mcp(app: Arc<App>, mut stop: watch::Receiver<bool>) {
    // Nonblocking duplicated stdin avoids a Tokio blocking reader that would prevent shutdown.
    let fd = unsafe { libc::dup(libc::STDIN_FILENO) };
    if fd < 0 {
        return;
    }
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        unsafe { libc::close(fd) };
        return;
    }
    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    let Ok(input) = AsyncFd::new(file) else {
        return;
    };
    let output_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
    if output_fd < 0 {
        return;
    }
    let flags = unsafe { libc::fcntl(output_fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(output_fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        unsafe { libc::close(output_fd) };
        return;
    }
    let output_file = unsafe { std::fs::File::from_raw_fd(output_fd) };
    let Ok(out) = AsyncFd::new(output_file) else {
        return;
    };
    let mut buffer = Vec::new();
    let mut initialized = false;
    loop {
        if *stop.borrow() {
            return;
        }
        while !buffer.contains(&b'\n') {
            let mut guard = tokio::select! { result=input.readable()=>match result { Ok(g)=>g, Err(_)=>return }, _=stop.changed()=>return };
            let mut chunk = [0u8; 4096];
            match guard.try_io(|inner| {
                let mut f = inner.get_ref();
                f.read(&mut chunk)
            }) {
                Ok(Ok(0)) => return,
                Ok(Ok(n)) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    if buffer.len() > INPUT_LIMIT {
                        return;
                    }
                }
                Ok(Err(_)) => return,
                Err(_) => continue,
            }
        }
        let n = buffer.iter().position(|b| *b == b'\n').unwrap() + 1;
        let line: Vec<u8> = buffer.drain(..n).collect();
        let v: Value = match serde_json::from_slice::<Unique>(&line) {
            Ok(v) => v.0,
            Err(_) => {
                let _=mcp_write(&out,b"{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,\"message\":\"Parse error\"}}\n").await;
                continue;
            }
        };
        let id = v.get("id").cloned();
        let method = v.get("method").and_then(Value::as_str).unwrap_or("");
        if !v.is_object()
            || v["jsonrpc"] != "2.0"
            || method.is_empty()
            || v.as_object().is_some_and(|o| {
                o.keys()
                    .any(|k| !["jsonrpc", "id", "method", "params"].contains(&k.as_str()))
            })
            || id
                .as_ref()
                .is_some_and(|i| !i.is_string() && !i.is_number() && !i.is_null())
        {
            let resp = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Invalid Request"}});
            let _ = mcp_write(&out, format!("{}\n", resp).as_bytes()).await;
            continue;
        }
        if method == "notifications/initialized" && id.is_none() {
            continue;
        }
        let valid_params = match method {
            "initialize" => v["params"].as_object().is_some_and(|p| {
                p.keys().all(|k| {
                    ["protocolVersion", "capabilities", "clientInfo"].contains(&k.as_str())
                }) && p.get("protocolVersion").and_then(Value::as_str).is_some()
                    && p.get("capabilities").is_some_and(Value::is_object)
                    && p.get("clientInfo").is_some_and(Value::is_object)
            }),
            "tools/list" => v
                .get("params")
                .is_none_or(|p| p.as_object().is_some_and(|o| o.is_empty())),
            _ => true,
        };
        if !valid_params {
            if let Some(id) = id {
                let resp = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Invalid params"}});
                let _ = mcp_write(&out, format!("{}\n", resp).as_bytes()).await;
            }
            continue;
        }
        let result = match method {
            "initialize" if !initialized && v["params"]["protocolVersion"] == "2025-06-18" => {
                initialized = true;
                json!({"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"work-engine-server","version":"0.1.0"}})
            }
            "tools/list" if initialized => {
                let mut listing = json!({"tools":[{"name":"list_sessions","description":"List sessions","inputSchema":{"type":"object","additionalProperties":false,"properties":{"query":{"type":"string"},"host":{"type":"string"},"limit":{"type":"integer"}}}},{"name":"read_session","description":"Read session","inputSchema":{"type":"object","additionalProperties":false,"properties":{"session_id":{"type":"string"},"entry_id":{"type":"string"},"before":{"type":"integer"},"after":{"type":"integer"},"tail":{"type":"integer"},"roles":{"type":"array","items":{"type":"string"}},"text_offset":{"type":"integer"},"max_chars":{"type":"integer"}},"required":["session_id"]}}]});
                if let Some(claims) = &app.repository_claims {
                    listing["tools"]
                        .as_array_mut()
                        .unwrap()
                        .extend(claims.tools());
                }
                if app.command_profile.is_some() {
                    let tools = listing["tools"].as_array_mut().unwrap();
                    tools.push(json!({"name":"submit_session_request","description":"Submit an already granted exact request","inputSchema":{"type":"object","additionalProperties":false,"properties":{"request_id":{"type":"string"},"grant_credential":{"type":"string"},"session_id":{"type":"string"},"workspace":{"type":"string"},"input_text":{"type":"string"}},"required":["request_id","grant_credential","session_id","workspace","input_text"]}}));
                    tools.push(json!({"name":"read_session_operation","description":"Read and reconcile an exact granted request","inputSchema":{"type":"object","additionalProperties":false,"properties":{"request_id":{"type":"string"},"grant_credential":{"type":"string"}},"required":["request_id","grant_credential"]}}));
                    tools.push(json!({"name":"read_session_result","description":"Read enriched result of an exact granted request","inputSchema":{"type":"object","additionalProperties":false,"properties":{"request_id":{"type":"string"},"grant_credential":{"type":"string"},"text_offset":{"type":"integer"},"max_chars":{"type":"integer"}},"required":["request_id","grant_credential"]}}));
                }
                listing
            }
            "tools/call" if initialized => {
                let p = &v["params"];
                if !p.is_object()
                    || p.as_object()
                        .unwrap()
                        .keys()
                        .any(|k| !["name", "arguments"].contains(&k.as_str()))
                    || !p["name"].is_string()
                {
                    if let Some(id) = id {
                        let resp = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Invalid params"}});
                        let _ = mcp_write(&out, format!("{}\n", resp).as_bytes()).await;
                    }
                    continue;
                }
                let op = p["name"].as_str().unwrap().to_string();
                let r = Request {
                    schema_version: 1,
                    operation: op,
                    arguments: p.get("arguments").cloned().unwrap_or(json!({})),
                    request_id: "mcp".into(),
                };
                let value = app.call(r).await;
                let error = value.get("error").is_some() || value.get("owner_error").is_some();
                json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":error})
            }
            _ => {
                if let Some(id) = id {
                    let resp = json!({"jsonrpc":"2.0","id":id,"error":{"code":if method=="initialize" {-32602}else{-32601},"message":"Unsupported method or parameters"}});
                    let _ = mcp_write(&out, format!("{}\n", resp).as_bytes()).await;
                }
                continue;
            }
        };
        if let Some(id) = id {
            let response = json!({"jsonrpc":"2.0","id":id,"result":result});
            if mcp_write(&out, format!("{}\n", response).as_bytes())
                .await
                .is_err()
            {
                return;
            }
        }
    }
}
fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("configuration required");
        std::process::exit(1);
    };
    // The query owner spawns its own helper. Clear ambient profile and provider
    // variables before creating any threads so that helper inherits no broad profile.
    for (name, _) in env::vars_os() {
        // SAFETY: this is the initial single-threaded entry before the runtime exists.
        unsafe { env::remove_var(name) };
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(run(path));
}

async fn run(path: String) {
    let (config, app) = match load(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("startup: {e}");
            std::process::exit(1)
        }
    };
    let listener = match TcpListener::bind(config.listen).await {
        Ok(l) => l,
        Err(_) => {
            eprintln!("bind failed");
            std::process::exit(1)
        }
    };
    let mut tasks = JoinSet::new();
    let connections = Arc::new(Semaphore::new(config.max_connections));
    let (stop_tx, stop_rx) = watch::channel(false);
    let mut mcp_task = if config.mcp {
        Some(tokio::spawn(mcp(app.clone(), stop_rx)))
    } else {
        None
    };
    let mut term =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal");
    let mut mcp_failed = false;
    loop {
        tokio::select! {
         _=tokio::signal::ctrl_c()=>break,
         _=term.recv()=>break,
         result=async {if let Some(t)=&mut mcp_task {t.await} else {std::future::pending().await}}=>{mcp_failed=result.is_err(); mcp_task=None; break},
         accepted=listener.accept()=>{if let Ok((stream,_))=accepted && let Ok(permit)=connections.clone().try_acquire_owned(){let app=app.clone();let addr=config.listen;let timeout=Duration::from_secs(config.request_timeout_seconds);tasks.spawn(async move {let _permit=permit;http(stream,addr,app,timeout).await;});}},
         Some(_)=tasks.join_next(), if !tasks.is_empty()=>{},
        }
    }
    drop(listener);
    let _ = stop_tx.send(true);
    if let Some(t) = mcp_task {
        mcp_failed = t.await.is_err();
    }
    while tasks.join_next().await.is_some() {}
    // A timed-out or disconnected transport can drop its join handle while the owner helper still runs.
    match tokio::time::timeout(
        Duration::from_secs(40),
        app.slots
            .clone()
            .acquire_many_owned(config.max_queries as u32),
    )
    .await
    {
        Ok(Ok(_)) if !mcp_failed => eprintln!("drain complete"),
        Ok(Ok(_)) => {
            eprintln!("drain complete; MCP task failed");
            std::process::exit(2)
        }
        _ => {
            eprintln!("drain deadline: unresolved helper custody");
            let _ = app
                .slots
                .clone()
                .acquire_many_owned(config.max_queries as u32)
                .await;
            std::process::exit(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Request, Unique};
    #[test]
    fn request_framing_rejects_duplicate_keys_at_any_depth() {
        let duplicate = br#"{"schema_version":1,"operation":"list_sessions","arguments":{"limit":1,"limit":2},"request_id":"x"}"#;
        assert!(serde_json::from_slice::<Unique>(duplicate).is_err());
        let valid =
            br#"{"schema_version":1,"operation":"list_sessions","arguments":{},"request_id":"x"}"#;
        let value: Unique = serde_json::from_slice(valid).unwrap();
        assert!(serde_json::from_value::<Request>(value.0).is_ok());
    }
    #[test]
    fn request_schema_rejects_client_profile_selection_and_extra_fields() {
        let request = br#"{"schema_version":1,"operation":"list_sessions","arguments":{},"request_id":"x","profile":"/tmp/profile"}"#;
        let value: Unique = serde_json::from_slice(request).unwrap();
        assert!(serde_json::from_value::<Request>(value.0).is_err());
    }
}
