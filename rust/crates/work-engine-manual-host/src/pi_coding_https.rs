//! Public HTTPS transport for the hosted Pi coding profile.
//!
//! Each payer has its own access, loaded by the embedder from sources it
//! names (never from the job, the journal or the container) and held only in
//! this process's memory. A route's endpoint fixes the origin and path; the
//! payer's credential kind must match it. Connections (TCP and TLS) are made
//! before gate admission; the request is written only inside the admitted
//! dispatch.
use super::hosted::{
    Connection, Endpoint, HostedProviderTransport, HostedRequest, HostedRoute, HostedRoutes,
    READ_IDLE_MS, Upstream, hold_secrets, left_ms, request_head,
};
use crate::{Error, Result, refuse};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const CREDENTIALS_SCHEMA: &str = "work-engine.pi-hosted-credentials.v1";

/// Where a payer's access comes from. Nonsecret; the embedder's own file.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CredentialSource {
    /// ChatGPT subscription access from Pi's auth store
    /// (`openai-codex.access`, `openai-codex.accountId`). Not refreshed here:
    /// expired access refuses.
    ChatgptPiAuth { path: PathBuf },
    /// API key from an environment variable of the embedder.
    ApiKeyEnv { variable: String },
    /// API key from an owner-only file (one line).
    ApiKeyFile { path: PathBuf },
    /// Qualification only: the account of a Pi auth store with access that
    /// the backend must refuse, to exercise a real definitive refusal.
    ChatgptInvalidForQualification { path: PathBuf },
    /// Claude subscription access from Claude Code's credentials file
    /// (`claudeAiOauth.accessToken`, `expiresAt`). The Host's own Claude Code
    /// keeps it fresh; it is read again before every connection and never
    /// refreshed here.
    ClaudeCodeAuth { path: PathBuf },
    /// Anthropic API key from an environment variable of the embedder.
    AnthropicApiKeyEnv { variable: String },
    /// Anthropic API key from an owner-only file (one line).
    AnthropicApiKeyFile { path: PathBuf },
    /// OpenRouter API key (`sk-or-...`) from an environment variable.
    OpenRouterApiKeyEnv { variable: String },
    /// OpenRouter API key (`sk-or-...`) from an owner-only file (one line).
    OpenRouterApiKeyFile { path: PathBuf },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialsFile {
    schema: String,
    payers: BTreeMap<String, CredentialSource>,
}

enum Credential {
    ChatGpt {
        access: String,
        account: String,
    },
    ApiKey(String),
    /// The current access and every earlier one this process read (all are
    /// screened).
    ClaudeOauth {
        path: PathBuf,
        access: String,
        earlier: Vec<String>,
    },
    AnthropicApiKey(String),
    OpenRouterApiKey(String),
}
impl Credential {
    fn endpoint(&self) -> Endpoint {
        match self {
            Self::ChatGpt { .. } => Endpoint::ChatGptCodex,
            Self::ApiKey(_) => Endpoint::OpenAiResponses,
            Self::ClaudeOauth { .. } => Endpoint::AnthropicOauth,
            Self::AnthropicApiKey(_) => Endpoint::AnthropicApi,
            Self::OpenRouterApiKey(_) => Endpoint::OpenRouter,
        }
    }
    fn secrets(&self) -> Vec<&[u8]> {
        match self {
            Self::ChatGpt { access, .. } => vec![access.as_bytes()],
            Self::ApiKey(key) | Self::AnthropicApiKey(key) | Self::OpenRouterApiKey(key) => {
                vec![key.as_bytes()]
            }
            Self::ClaudeOauth {
                access, earlier, ..
            } => std::iter::once(access)
                .chain(earlier)
                .map(String::as_bytes)
                .collect(),
        }
    }
}
/// Owner-only regular file of the current user, at most `limit` bytes.
fn owner_only(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let m = std::fs::symlink_metadata(path)?;
    if !m.is_file()
        || m.len() > limit
        || m.permissions().mode() & 0o077 != 0
        || m.uid() != unsafe { libc::geteuid() }
    {
        return refuse(format!(
            "{} must be a small owner-only regular file",
            path.display()
        ));
    }
    Ok(std::fs::read(path)?)
}
fn valid_token(v: &str) -> bool {
    (16..=8192).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_graphic())
}
fn pi_auth(path: &Path) -> Result<(String, String)> {
    let v: serde_json::Value = serde_json::from_slice(&owner_only(path, 1 << 20)?)?;
    let e = &v["openai-codex"];
    let access = e["access"].as_str().unwrap_or("").to_string();
    let account = e["accountId"].as_str().unwrap_or("").to_string();
    let expires = e["expires"].as_i64().unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Refused("clock".into()))?
        .as_millis() as i64;
    if !valid_token(&access)
        || account.is_empty()
        || account.len() > 128
        || !account
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return refuse("Pi auth store has no usable openai-codex access");
    }
    if expires <= now + 600_000 {
        return refuse("openai-codex access expires within 10 minutes; refresh it first");
    }
    Ok((access, account))
}

/// Claude Code's subscription access, valid for at least `margin_ms`.
fn claude_auth(path: &Path, margin_ms: i64) -> Result<String> {
    let v: serde_json::Value = serde_json::from_slice(&owner_only(path, 1 << 20)?)?;
    let e = &v["claudeAiOauth"];
    let access = e["accessToken"].as_str().unwrap_or("").to_string();
    let expires = e["expiresAt"].as_i64().unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Refused("clock".into()))?
        .as_millis() as i64;
    if !valid_token(&access) || !access.starts_with("sk-ant-oat") {
        return refuse("Claude Code credentials have no usable subscription access");
    }
    if expires <= now + margin_ms {
        return refuse("Claude subscription access expires too soon; refresh it first");
    }
    Ok(access)
}
fn key_from_env(variable: &str) -> Result<String> {
    let key = std::env::var(variable).unwrap_or_default();
    if !valid_token(&key) {
        return refuse(format!("API key variable {variable} unusable"));
    }
    Ok(key)
}
fn key_from_file(path: &Path) -> Result<String> {
    let key = String::from_utf8(owner_only(path, 4096)?)
        .map_err(|_| Error::Refused("API key file encoding".into()))?
        .trim()
        .to_string();
    if !valid_token(&key) {
        return refuse("API key file unusable");
    }
    Ok(key)
}

/// Host-held HTTPS transport to the fixed public endpoints.
pub struct HttpsTransport {
    credentials: BTreeMap<String, Credential>,
    tls: Arc<rustls::ClientConfig>,
    identity: serde_json::Value,
}
impl HttpsTransport {
    /// Load each payer's access from the embedder's credentials file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = owner_only(path, 65536)?;
        let file: CredentialsFile = serde_json::from_slice(&raw)?;
        if file.schema != CREDENTIALS_SCHEMA || file.payers.is_empty() || file.payers.len() > 8 {
            return refuse("hosted credentials file invalid");
        }
        let mut credentials = BTreeMap::new();
        let kinds: BTreeMap<String, &'static str> = file
            .payers
            .iter()
            .map(|(payer, source)| {
                (
                    payer.clone(),
                    match source {
                        CredentialSource::ChatgptPiAuth { .. } => "chatgpt-pi-auth",
                        CredentialSource::ApiKeyEnv { .. } => "api-key-env",
                        CredentialSource::ApiKeyFile { .. } => "api-key-file",
                        CredentialSource::ChatgptInvalidForQualification { .. } => {
                            "chatgpt-invalid-for-qualification"
                        }
                        CredentialSource::ClaudeCodeAuth { .. } => "claude-code-auth",
                        CredentialSource::AnthropicApiKeyEnv { .. } => "anthropic-api-key-env",
                        CredentialSource::AnthropicApiKeyFile { .. } => "anthropic-api-key-file",
                        CredentialSource::OpenRouterApiKeyEnv { .. } => "openrouter-api-key-env",
                        CredentialSource::OpenRouterApiKeyFile { .. } => "openrouter-api-key-file",
                    },
                )
            })
            .collect();
        let identity = serde_json::json!({
            "transport": "https",
            "credentials_file_sha256": crate::digest(&raw),
            "payer_sources": kinds,
        });
        for (payer, source) in file.payers {
            let credential = match source {
                CredentialSource::ChatgptPiAuth { path } => {
                    let (access, account) = pi_auth(&path)?;
                    Credential::ChatGpt { access, account }
                }
                CredentialSource::ApiKeyEnv { variable } => {
                    Credential::ApiKey(key_from_env(&variable)?)
                }
                CredentialSource::ApiKeyFile { path } => Credential::ApiKey(key_from_file(&path)?),
                CredentialSource::ClaudeCodeAuth { path } => Credential::ClaudeOauth {
                    access: claude_auth(&path, 600_000)?,
                    path,
                    earlier: Vec::new(),
                },
                CredentialSource::AnthropicApiKeyEnv { variable } => {
                    Credential::AnthropicApiKey(key_from_env(&variable)?)
                }
                CredentialSource::AnthropicApiKeyFile { path } => {
                    Credential::AnthropicApiKey(key_from_file(&path)?)
                }
                CredentialSource::OpenRouterApiKeyEnv { variable } => {
                    Credential::OpenRouterApiKey(key_from_env(&variable)?)
                }
                CredentialSource::OpenRouterApiKeyFile { path } => {
                    Credential::OpenRouterApiKey(key_from_file(&path)?)
                }
                CredentialSource::ChatgptInvalidForQualification { path } => {
                    let (_, account) = pi_auth(&path)?;
                    let mut random = [0u8; 24];
                    std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
                    let access = format!(
                        "invalid-for-qualification-{}",
                        random
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    );
                    Credential::ChatGpt { access, account }
                }
            };
            credentials.insert(payer, credential);
        }
        Self::new(credentials, identity)
    }
    fn new(credentials: BTreeMap<String, Credential>, identity: serde_json::Value) -> Result<Self> {
        use rustls_platform_verifier::ConfigVerifierExt;
        let tls = rustls::ClientConfig::with_platform_verifier()
            .map_err(|e| Error::Refused(format!("TLS configuration: {e}")))?;
        Ok(Self {
            credentials,
            tls: Arc::new(tls),
            identity,
        })
    }
    fn credential(&self, route: &HostedRoute) -> Result<(Endpoint, &Credential)> {
        let endpoint = Endpoint::parse(&route.endpoint)
            .ok_or_else(|| Error::Refused("route endpoint".into()))?;
        let credential = self
            .credentials
            .get(&route.payer)
            .ok_or_else(|| Error::Refused("no access held for the route's payer".into()))?;
        if credential.endpoint() != endpoint {
            return refuse("payer credential kind does not match the route endpoint");
        }
        Ok((endpoint, credential))
    }
}
impl HostedProviderTransport for HttpsTransport {
    fn identity(&self) -> serde_json::Value {
        self.identity.clone()
    }
    fn check(&self, routes: &HostedRoutes) -> Result<()> {
        for route in &routes.routes {
            self.credential(route)?;
        }
        Ok(())
    }
    fn connect(&mut self, route: &HostedRoute, deadline_ms: i64) -> Result<Connection> {
        let (endpoint, _) = self.credential(route)?;
        // Subscription access is read again (it may have been refreshed since)
        // and must outlive this attempt's deadline window by its margin.
        if let Some(Credential::ClaudeOauth {
            path,
            access,
            earlier,
        }) = self.credentials.get_mut(&route.payer)
        {
            let current = claude_auth(path, 300_000)?;
            if current != *access {
                earlier.push(std::mem::replace(access, current));
            }
        }
        let host = endpoint.host();
        let address = (host, 443)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::Refused("provider host did not resolve".into()))?;
        let mut tcp = TcpStream::connect_timeout(&address, left_ms(deadline_ms, 5000)?)?;
        tcp.set_write_timeout(Some(left_ms(deadline_ms, 5000)?))?;
        tcp.set_read_timeout(Some(left_ms(deadline_ms, 5000)?))?;
        let name = rustls::pki_types::ServerName::try_from(host)
            .map_err(|_| Error::Refused("server name".into()))?;
        let mut tls = rustls::ClientConnection::new(self.tls.clone(), name)
            .map_err(|e| Error::Refused(format!("TLS: {e}")))?;
        while tls.is_handshaking() {
            tls.complete_io(&mut tcp)?;
        }
        tcp.set_read_timeout(Some(left_ms(deadline_ms, READ_IDLE_MS)?))?;
        let killer = tcp.try_clone()?;
        Ok(Connection {
            stream: Box::new(rustls::StreamOwned::new(tls, tcp)),
            abort: Box::new(move || {
                let _ = killer.shutdown(Shutdown::Both);
            }),
        })
    }
    fn send(
        &mut self,
        connection: Connection,
        route: &HostedRoute,
        request: &HostedRequest,
    ) -> Result<Upstream> {
        let (endpoint, credential) = self.credential(route)?;
        let head = match credential {
            Credential::ChatGpt { access, account } => request_head(
                endpoint,
                &[
                    ("Authorization", &format!("Bearer {access}")),
                    ("chatgpt-account-id", account),
                ],
                request,
            ),
            Credential::ApiKey(key) => request_head(
                endpoint,
                &[("Authorization", &format!("Bearer {key}"))],
                request,
            ),
            Credential::ClaudeOauth { access, .. } => request_head(
                endpoint,
                &[("Authorization", &format!("Bearer {access}"))],
                request,
            ),
            Credential::AnthropicApiKey(key) => {
                request_head(endpoint, &[("x-api-key", key)], request)
            }
            Credential::OpenRouterApiKey(key) => {
                request_head(endpoint, &[("Authorization", &format!("Bearer {key}"))], request)
            }
        };
        let Connection { mut stream, abort } = connection;
        stream.write_all(head.as_bytes())?;
        stream.write_all(&request.body)?;
        stream.flush()?;
        Ok((Box::new(stream), abort))
    }
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
        let secrets: Vec<&[u8]> = self
            .credentials
            .values()
            .flat_map(Credential::secrets)
            .collect();
        hold_secrets(&secrets, pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn transport() -> HttpsTransport {
        HttpsTransport::new(
            BTreeMap::from([
                (
                    "subscription".to_string(),
                    Credential::ChatGpt {
                        access: "chatgpt-access-0123456789".into(),
                        account: "acct-1".into(),
                    },
                ),
                (
                    "api".to_string(),
                    Credential::ApiKey("sk-test-0123456789abcdef".into()),
                ),
                (
                    "claude".to_string(),
                    Credential::ClaudeOauth {
                        path: PathBuf::from("/nonexistent"),
                        access: "sk-ant-oat01-current-0123456789".into(),
                        earlier: vec!["sk-ant-oat01-earlier-0123456789".into()],
                    },
                ),
                (
                    "anthropic-api".to_string(),
                    Credential::AnthropicApiKey("sk-ant-api03-0123456789abcdef".into()),
                ),
                (
                    "openrouter".to_string(),
                    Credential::OpenRouterApiKey("sk-or-v1-0123456789abcdef".into()),
                ),
            ]),
            serde_json::json!({}),
        )
        .unwrap()
    }
    fn route(id: &str, payer: &str, endpoint: &str) -> HostedRoute {
        HostedRoute {
            id: id.into(),
            payer: payer.into(),
            endpoint: endpoint.into(),
        }
    }
    #[test]
    fn openrouter_routes_need_their_openrouter_access_and_screen_it() {
        let t = transport();
        let ok = HostedRoutes {
            routes: vec![HostedRoute {
                id: "primary".into(),
                payer: "openrouter".into(),
                endpoint: "https:openrouter".into(),
            }],
            max_attempts_per_request: 1,
        };
        t.check(&ok).unwrap();
        for bad in [
            route("p", "api", "https:openrouter"), // an OpenAI key is not OpenRouter access
            route("p", "openrouter", "https:openai-responses"), // wrong endpoint for the key
            route("p", "missing", "https:openrouter"),
        ] {
            let routes = HostedRoutes {
                routes: vec![bad],
                max_attempts_per_request: 1,
            };
            assert!(t.check(&routes).is_err());
        }
        // The OpenRouter key is screened from everything relayed to the container.
        assert!(t.hold(b"x sk-or-v1-0123456789abcdef y").is_err());
        // A partial key prefix is not surfaced; its tail is held back.
        assert!(t.hold(b"x sk-or-").unwrap() > 0);
    }
    #[test]
    fn openrouter_heads_use_the_fixed_origin_and_allowlisted_headers() {
        let request = HostedRequest {
            request: 4,
            attempt: 1,
            headers: BTreeMap::from([
                (
                    "x-requested-with".to_string(),
                    "pi".to_string(),
                ),
                ("openai-organization".to_string(), "org-x".to_string()),
                ("accept".to_string(), "text/event-stream".to_string()),
            ]),
            body: b"{}".to_vec(),
            wire: crate::pi_stream::Wire::Completions,
            beta: false,
        };
        let head = request_head(
            Endpoint::OpenRouter,
            &[("Authorization", "Bearer sk-or-v1-0123456789abcdef")],
            &request,
        );
        assert!(head
            .starts_with("POST /api/v1/chat/completions HTTP/1.1\r\nHost: openrouter.ai\r\n"));
        assert!(head.contains("Authorization: Bearer sk-or-v1-0123456789abcdef\r\n"));
        assert!(head.contains("x-requested-with: pi\r\n"));
        assert!(!head.contains("openai-organization"));
    }
    #[test]
    fn each_route_needs_its_payers_access_of_the_matching_kind() {
        let t = transport();
        let ok = HostedRoutes {
            routes: vec![
                route("primary", "subscription", "https:chatgpt-codex"),
                route("secondary", "api", "https:openai-responses"),
            ],
            max_attempts_per_request: 2,
        };
        t.check(&ok).unwrap();
        for bad in [
            route("primary", "api", "https:chatgpt-codex"),
            route("primary", "subscription", "https:openai-responses"),
            route("primary", "unknown", "https:openai-responses"),
            route("primary", "subscription", "loopback:9"),
        ] {
            let routes = HostedRoutes {
                routes: vec![bad],
                max_attempts_per_request: 1,
            };
            assert!(t.check(&routes).is_err());
        }
    }
    #[test]
    fn messages_routes_need_their_anthropic_access() {
        let t = transport();
        let ok = HostedRoutes {
            routes: vec![
                route("primary", "claude", "https:anthropic-oauth"),
                route("secondary", "anthropic-api", "https:anthropic-api"),
            ],
            max_attempts_per_request: 2,
        };
        t.check(&ok).unwrap();
        for bad in [
            route("primary", "claude", "https:anthropic-api"),
            route("primary", "anthropic-api", "https:anthropic-oauth"),
            route("primary", "subscription", "https:anthropic-oauth"),
            route("primary", "claude", "https:chatgpt-codex"),
        ] {
            let routes = HostedRoutes {
                routes: vec![bad],
                max_attempts_per_request: 1,
            };
            assert!(t.check(&routes).is_err());
        }
        // Current and earlier subscription access and the API key are screened.
        assert!(t.hold(b"x sk-ant-oat01-current-0123456789").is_err());
        assert!(t.hold(b"x sk-ant-oat01-earlier-0123456789").is_err());
        assert!(t.hold(b"x sk-ant-api03-0123456789abcdef").is_err());
    }
    #[test]
    fn messages_heads_keep_the_beta_query_and_claude_code_headers_only() {
        let mut request = HostedRequest {
            request: 1,
            attempt: 1,
            headers: BTreeMap::from([
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
                ("anthropic-beta".to_string(), "x".to_string()),
                ("x-app".to_string(), "cli".to_string()),
                ("x-stainless-retry-count".to_string(), "0".to_string()),
                ("originator".to_string(), "pi".to_string()),
                ("x-unknown".to_string(), "u".to_string()),
            ]),
            body: b"{}".to_vec(),
            wire: crate::pi_stream::Wire::Messages,
            beta: true,
        };
        let oauth = request_head(
            Endpoint::AnthropicOauth,
            &[("Authorization", "Bearer t")],
            &request,
        );
        assert!(
            oauth
                .starts_with("POST /v1/messages?beta=true HTTP/1.1\r\nHost: api.anthropic.com\r\n")
        );
        for kept in [
            "anthropic-version: 2023-06-01\r\n",
            "anthropic-beta: x\r\n",
            "x-app: cli\r\n",
            "x-stainless-retry-count: 0\r\n",
        ] {
            assert!(oauth.contains(kept), "{kept}");
        }
        assert!(!oauth.contains("originator") && !oauth.contains("x-unknown"));
        request.beta = false;
        let api = request_head(Endpoint::AnthropicApi, &[("x-api-key", "k")], &request);
        assert!(api.starts_with("POST /v1/messages HTTP/1.1\r\nHost: api.anthropic.com\r\n"));
        assert!(api.contains("x-api-key: k\r\n"));
        let looped = request_head(
            Endpoint::Loopback(std::net::SocketAddr::from(([127, 0, 0, 1], 9))),
            &[],
            &request,
        );
        assert!(looped.starts_with("POST /v1/messages HTTP/1.1\r\nHost: localhost\r\n"));
    }
    #[test]
    fn every_payers_access_is_screened() {
        let t = transport();
        assert!(t.hold(b"x chatgpt-access-0123456789 y").is_err());
        assert!(t.hold(b"x sk-test-0123456789abcdef").is_err());
        assert_eq!(t.hold(b"data sk-te").unwrap(), 5);
        assert_eq!(t.hold(b"data chatgpt-acc").unwrap(), 11);
        assert_eq!(t.hold(b"clean").unwrap(), 0);
    }
    #[test]
    fn heads_use_fixed_origins_and_allowlisted_headers() {
        let request = HostedRequest {
            request: 3,
            attempt: 2,
            headers: BTreeMap::from([
                ("originator".to_string(), "pi".to_string()),
                ("session-id".to_string(), "s".to_string()),
                ("openai-organization".to_string(), "org-x".to_string()),
                ("openai-project".to_string(), "proj-x".to_string()),
                ("accept".to_string(), "text/event-stream".to_string()),
            ]),
            body: b"{}".to_vec(),
            wire: crate::pi_stream::Wire::Responses,
            beta: false,
        };
        let codex = request_head(
            Endpoint::ChatGptCodex,
            &[("Authorization", "Bearer a")],
            &request,
        );
        assert!(
            codex
                .starts_with("POST /backend-api/codex/responses HTTP/1.1\r\nHost: chatgpt.com\r\n")
        );
        assert!(codex.contains("originator: pi\r\n") && codex.contains("session-id: s\r\n"));
        let api = request_head(
            Endpoint::OpenAiResponses,
            &[("Authorization", "Bearer k")],
            &request,
        );
        assert!(api.starts_with("POST /v1/responses HTTP/1.1\r\nHost: api.openai.com\r\n"));
        assert!(!api.contains("originator") && !api.contains("session-id"));
        for head in [&codex, &api] {
            assert!(!head.contains("openai-organization") && !head.contains("openai-project"));
            assert!(head.contains("accept: text/event-stream\r\n"));
            assert!(head.contains("X-Request-Id: 3-2\r\n"));
        }
    }
}
