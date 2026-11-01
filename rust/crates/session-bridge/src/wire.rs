use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const HEADER: &str = include_str!("runtime-envelope-v1.txt");
pub const LIMIT: usize = 32 * 1024;

#[derive(Serialize)]
struct Envelope<'a> {
    request_id: &'a str,
    session_id: &'a str,
    workspace: &'a str,
    payload_sha256: &'a str,
    payload: &'a str,
}

pub fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

pub fn valid_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|u| u.hyphenated().to_string() == value)
}

pub fn wire(
    request: &str,
    session: &str,
    workspace: &str,
    payload: &str,
) -> Result<(String, String, String), ()> {
    if !valid_uuid(request)
        || !valid_uuid(session)
        || payload.len() > LIMIT
        || payload.contains('\0')
    {
        return Err(());
    }
    let payload_sha = digest(payload.as_bytes());
    let envelope = Envelope {
        request_id: request,
        session_id: session,
        workspace,
        payload_sha256: &payload_sha,
        payload,
    };
    let text = format!(
        "{}{}\n",
        HEADER,
        serde_json::to_string(&envelope).map_err(|_| ())?
    );
    let wire_sha = digest(text.as_bytes());
    Ok((text, payload_sha, wire_sha))
}

pub fn operation_digest(
    request: &str,
    session: &str,
    workspace: &str,
    payload_hash: &str,
    wire_hash: &str,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        protocol: &'static str,
        envelope: &'static str,
        profile: &'static str,
        request_id: &'a str,
        session_id: &'a str,
        workspace: &'a str,
        payload_sha256: &'a str,
        wire_sha256: &'a str,
    }
    let identity = Identity {
        protocol: "session-bridge-rpc-v1",
        envelope: "runtime-envelope-v1",
        profile: "dedicated-loaded-session-v1",
        request_id: request,
        session_id: session,
        workspace,
        payload_sha256: payload_hash,
        wire_sha256: wire_hash,
    };
    digest(&serde_json::to_vec(&identity).expect("serializing string identity cannot fail"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub store_id: String,
    pub database: String,
    pub lock_dir: String,
    pub native_socket: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rpc {
    pub schema_version: u32,
    pub method: String,
    pub request_id: String,
    pub grant_credential: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub input_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_header_and_exact_unicode_wire_vector() {
        assert_eq!(
            digest(HEADER.as_bytes()),
            "f43783fe6f277ce48486cd9141959298691113d24461ce705c32d48c2c27dc3f"
        );
        let request = "11111111-1111-4111-8111-111111111111";
        let session = "22222222-2222-4222-8222-222222222222";
        let (_, payload, wire_hash) =
            wire(request, session, "/workspace", "quote \"雪\"\n").unwrap();
        assert_eq!(
            payload,
            "e0b1de5676edc07ece29e340214f092854151f6988aa1063816fb91812f26625"
        );
        assert_eq!(
            wire_hash,
            "0f4530007ec3be893da75493e1990506c01d1a1bc6fe13952fd2a7360beaf7e7"
        );
        assert_eq!(
            operation_digest(request, session, "/workspace", &payload, &wire_hash),
            "6759d7c0eaf4bfa0155138841501e4c6878074697262f19d966fa55468306a9e"
        );
        assert!(wire(request, session, "/workspace", "bad\0input").is_err());
        assert!(wire(request, session, "/workspace", &"a".repeat(LIMIT + 1)).is_err());
    }
}
