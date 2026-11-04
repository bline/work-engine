//! Local client for the application server HTTP envelope.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u8,
    request_id: String,
    server_identity: String,
    server_build: String,
    owner_result: Option<Value>,
    owner_error: Option<ErrorBody>,
    error: Option<ErrorBody>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    code: String,
}

/// The endpoint and credential file are trusted local operator configuration.
pub struct LocalClient {
    endpoint: SocketAddr,
    credential_file: String,
    timeout: Duration,
}
impl LocalClient {
    pub fn new(
        endpoint: SocketAddr,
        credential_file: String,
        timeout: Duration,
    ) -> Result<Self, &'static str> {
        if !endpoint.ip().is_loopback() || endpoint.port() == 0 || timeout.is_zero() {
            return Err("invalid_client_config");
        }
        Ok(Self {
            endpoint,
            credential_file,
            timeout,
        })
    }
    /// Returns the complete server envelope, including owner errors, without rewriting it.
    pub fn query(
        &self,
        operation: &str,
        arguments: Value,
        request_id: &str,
    ) -> Result<Value, &'static str> {
        if !matches!(operation, "list_sessions" | "read_session") {
            return Err("invalid_request");
        }
        self.call(operation, arguments, request_id)
    }

    /// Calls a granted command or its public reconciliation reads. The server capability
    /// and the independent domain grant both remain authoritative.
    pub fn command(
        &self,
        operation: &str,
        arguments: Value,
        request_id: &str,
    ) -> Result<Value, &'static str> {
        if !matches!(
            operation,
            "submit_session_request" | "read_session_operation" | "read_session_result"
        ) {
            return Err("invalid_request");
        }
        self.call(operation, arguments, request_id)
    }

    fn call(
        &self,
        operation: &str,
        arguments: Value,
        request_id: &str,
    ) -> Result<Value, &'static str> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("invalid_request");
        }
        let token = session_query::read_file(&self.credential_file, 4096, true)?;
        if token.len() < 32 || token.iter().any(|x| !x.is_ascii_graphic()) {
            return Err("invalid_credential");
        }
        let body=serde_json::to_vec(&json!({"schema_version":1,"operation":operation,"arguments":arguments,"request_id":request_id})).map_err(|_|"invalid_request")?;
        if body.len() > session_query::INPUT_LIMIT {
            return Err("input_too_large");
        }
        let mut stream = TcpStream::connect_timeout(&self.endpoint, self.timeout)
            .map_err(|_| "transport_unavailable")?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|_| "transport_unavailable")?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|_| "transport_unavailable")?;
        let host = if self.endpoint.is_ipv6() {
            format!("[{}]:{}", self.endpoint.ip(), self.endpoint.port())
        } else {
            self.endpoint.to_string()
        };
        let header = format!(
            "POST /v1/session-queries HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            std::str::from_utf8(&token).map_err(|_| "invalid_credential")?,
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .and_then(|_| stream.write_all(&body))
            .map_err(|_| "transport_unavailable")?;
        let mut data = Vec::new();
        stream
            .take(1024 * 1024 + 16385)
            .read_to_end(&mut data)
            .map_err(|_| "transport_unavailable")?;
        if data.len() > 1024 * 1024 + 16384 {
            return Err("output_too_large");
        }
        let split = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or("invalid_response")?;
        if split > 16384 || !data.starts_with(b"HTTP/1.1 200 ") {
            return Err("transport_rejected");
        }
        let headers = std::str::from_utf8(&data[..split]).map_err(|_| "invalid_response")?;
        let mut length = None;
        let mut content_type = None;
        for line in headers.split("\r\n").skip(1) {
            let (key, value) = line.split_once(':').ok_or("invalid_response")?;
            match key.to_ascii_lowercase().as_str() {
                "content-length" if length.is_none() => {
                    length = Some(
                        value
                            .trim()
                            .parse::<usize>()
                            .map_err(|_| "invalid_response")?,
                    )
                }
                "content-type" if content_type.is_none() => content_type = Some(value.trim()),
                "content-length" | "content-type" | "transfer-encoding" => {
                    return Err("invalid_response");
                }
                _ => {}
            }
        }
        let body = &data[split + 4..];
        if length != Some(body.len()) || content_type != Some("application/json") {
            return Err("invalid_response");
        }
        let envelope: Envelope = serde_json::from_slice(body).map_err(|_| "invalid_response")?;
        let outcomes = usize::from(envelope.owner_result.is_some())
            + usize::from(envelope.owner_error.is_some())
            + usize::from(envelope.error.is_some());
        if envelope.schema_version != 1
            || envelope.request_id != request_id
            || outcomes != 1
            || !envelope.server_identity.starts_with("sha256:")
            || !envelope.server_build.starts_with("sha256:")
            || envelope
                .owner_error
                .as_ref()
                .is_some_and(|e| e.code.is_empty())
            || envelope.error.as_ref().is_some_and(|e| e.code.is_empty())
        {
            return Err("invalid_response");
        }
        serde_json::from_slice(body).map_err(|_| "invalid_response")
    }
}
