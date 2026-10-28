//! Fixed-origin SIWC HTTPS. The only alternate peer and root exist in unit tests.
use crate::{
    BearerAccess, CallbackExchange, DiscoveredModel, Issuer, RefreshFailure, RefreshGrant,
    RevocationGrant, TokenResponse,
};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    redirect::Policy,
    Client, Response,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
use thiserror::Error;
use url::Url;

const ISSUER: &str = "https://auth.openai.com";
const METADATA: &str = "https://auth.openai.com/.well-known/openid-configuration";
const AUTHORIZE: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const MODELS: &str = "https://api.openai.com/v1/models";
// Bound both catalog acquisition and later consumption of the retained bytes.
pub(crate) const MODEL_BODY_LIMIT: usize = 2 * 1024 * 1024;
const RESPONSES: &str = "https://api.openai.com/v1/responses";
const RESOURCE: &str = "https://api.openai.com/v1";
const MAX_HEADERS: usize = 16 * 1024;

#[derive(Debug, Error)]
pub enum PublicHttpsError {
    #[error("public HTTPS policy rejected issuer metadata or endpoint")]
    Policy,
    #[error("public HTTPS request failed")]
    Transport,
    #[error("public HTTPS deadline exceeded")]
    Deadline,
    #[error("public HTTPS rejected HTTP status {0}")]
    HttpStatus(u16),
    #[error("public HTTPS response headers exceeded bound")]
    HeaderBound,
    #[error("public HTTPS body exceeded bound")]
    BodyBound,
    #[error("public HTTPS response rejected")]
    Response,
    #[error("public HTTPS model response is not JSON")]
    ModelJson,
    #[error("public HTTPS model response has invalid schema")]
    ModelSchema,
    #[error("public HTTPS model count exceeded bound")]
    Bound,
}

#[derive(Clone)]
pub struct PublicHttps {
    client: Client,
}

pub struct TrustedIssuer {
    pub jwks: String,
}

pub struct ModelResponse {
    pub models: Vec<DiscoveredModel>,
    pub response_sha256: String,
    pub response_bytes: usize,
    pub raw_body: Vec<u8>,
}

pub fn local_issuer() -> Result<Issuer, PublicHttpsError> {
    Issuer::new(
        ISSUER,
        AUTHORIZE,
        TOKEN,
        "https://auth.openai.com/api/accounts/oauth/revoke",
    )
    .map_err(|_| PublicHttpsError::Policy)
}

#[derive(Deserialize)]
struct Metadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    revocation_endpoint: String,
    jwks_uri: String,
}

impl PublicHttps {
    pub fn new() -> Result<Self, PublicHttpsError> {
        let client = Self::client(None, None)?;
        Ok(Self { client })
    }

    fn client(
        #[cfg_attr(not(test), allow(unused_variables))] root: Option<reqwest::Certificate>,
        #[cfg_attr(not(test), allow(unused_variables))] peer: Option<std::net::SocketAddr>,
    ) -> Result<Client, PublicHttpsError> {
        #[allow(unused_mut)]
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .https_only(true)
            .http1_only()
            .http1_max_headers(64)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(30));
        #[cfg(test)]
        {
            if let Some(root) = root {
                builder = builder.add_root_certificate(root);
            }
            if let Some(peer) = peer {
                builder = builder
                    .resolve("auth.openai.com", peer)
                    .resolve("api.openai.com", peer);
            }
        }
        Ok(builder.build().map_err(|_| PublicHttpsError::Policy)?)
    }

    #[cfg(test)]
    pub(crate) fn synthetic(root_der: &[u8], peer: std::net::SocketAddr) -> Self {
        let root = reqwest::Certificate::from_der(root_der).unwrap();
        Self {
            client: Self::client(Some(root), Some(peer)).unwrap(),
        }
    }

    pub async fn discover(&self) -> Result<TrustedIssuer, PublicHttpsError> {
        let bytes = self
            .get_bounded(METADATA, 64 * 1024, Duration::from_secs(5))
            .await?;
        let metadata: Metadata =
            serde_json::from_slice(&bytes).map_err(|_| PublicHttpsError::Policy)?;
        if metadata.issuer != ISSUER
            || metadata.authorization_endpoint != AUTHORIZE
            || metadata.token_endpoint != TOKEN
            || metadata.revocation_endpoint != "https://auth.openai.com/api/accounts/oauth/revoke"
            || !trusted_auth_path(&metadata.jwks_uri, "/.well-known/")
        {
            return Err(PublicHttpsError::Policy);
        }
        let jwks = self
            .get_bounded(&metadata.jwks_uri, 64 * 1024, Duration::from_secs(5))
            .await?;
        let jwks: serde_json::Value =
            serde_json::from_slice(&jwks).map_err(|_| PublicHttpsError::Policy)?;
        if !jwks
            .get("keys")
            .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        {
            return Err(PublicHttpsError::Policy);
        }
        Ok(TrustedIssuer {
            jwks: jwks.to_string(),
        })
    }

    pub(crate) async fn get_bounded(
        &self,
        url: &str,
        limit: usize,
        deadline: Duration,
    ) -> Result<Vec<u8>, PublicHttpsError> {
        tokio::time::timeout(deadline, async {
            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|_| PublicHttpsError::Transport)?;
            read_bounded(response, limit, ReadTimeout::Transport).await
        })
        .await
        .map_err(|_| PublicHttpsError::Transport)?
    }

    pub async fn exchange(
        &self,
        grant: &CallbackExchange,
    ) -> Result<TokenResponse, PublicHttpsError> {
        if grant.token_endpoint() != TOKEN || grant.resource() != RESOURCE {
            return Err(PublicHttpsError::Policy);
        }
        let bytes = tokio::time::timeout(Duration::from_secs(10), async {
            let response = self
                .client
                .post(TOKEN)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", grant.client_id()),
                    ("code", grant.code()),
                    ("code_verifier", grant.code_verifier()),
                    ("redirect_uri", grant.redirect_uri()),
                    ("resource", grant.resource()),
                ])
                .send()
                .await
                .map_err(|_| PublicHttpsError::Transport)?;
            read_bounded(response, 64 * 1024, ReadTimeout::Transport).await
        })
        .await
        .map_err(|_| PublicHttpsError::Transport)??;
        TokenResponse::from_json(&bytes).map_err(|_| PublicHttpsError::Response)
    }

    /// Return Ambiguous for any uncertain POST or malformed success. A terminal
    /// state requires a complete, documented machine-readable rejection.
    pub async fn refresh(&self, grant: &RefreshGrant) -> Result<TokenResponse, RefreshFailure> {
        if grant.resource != RESOURCE
            || grant.client_id.is_empty()
            || grant.refresh_token().is_empty()
        {
            return Err(RefreshFailure::Ambiguous);
        }
        let (status, bytes) = tokio::time::timeout(Duration::from_secs(10), async {
            let response = self
                .client
                .post(TOKEN)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("client_id", grant.client_id.as_str()),
                    ("refresh_token", grant.refresh_token()),
                    ("resource", grant.resource.as_str()),
                ])
                .send()
                .await
                .map_err(|_| RefreshFailure::Ambiguous)?;
            let status = response.status().as_u16();
            let bytes = read_refresh_body(response, 64 * 1024).await?;
            Ok::<_, RefreshFailure>((status, bytes))
        })
        .await
        .map_err(|_| RefreshFailure::Ambiguous)??;
        if status == 200 {
            return TokenResponse::from_json(&bytes).map_err(|_| RefreshFailure::Ambiguous);
        }
        if status == 400 || status == 401 {
            let code = serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_owned));
            if matches!(
                code.as_deref(),
                Some(
                    "invalid_grant"
                        | "invalid_refresh_token"
                        | "token_expired"
                        | "refresh_token_expired"
                        | "refresh_token_invalidated"
                        | "refresh_token_reused"
                )
            ) {
                return Err(RefreshFailure::Terminal);
            }
            if code.as_deref() == Some("invalid_client") {
                return Err(RefreshFailure::Configuration);
            }
        }
        Err(RefreshFailure::Ambiguous)
    }

    pub async fn models(&self, bearer: &BearerAccess) -> Result<ModelResponse, PublicHttpsError> {
        let bytes = tokio::time::timeout(Duration::from_secs(5), async {
            let response = self
                .client
                .get(MODELS)
                .header(AUTHORIZATION, bearer.authorization_value())
                .send()
                .await
                .map_err(classify_request_error)?;
            read_bounded(response, MODEL_BODY_LIMIT, ReadTimeout::Deadline).await
        })
        .await
        .map_err(|_| PublicHttpsError::Deadline)??;
        let response_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let response_bytes = bytes.len();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| PublicHttpsError::ModelJson)?;
        let models = json
            .get("models")
            .and_then(|v| v.as_array())
            .ok_or(PublicHttpsError::ModelSchema)?;
        if models.len() > 512 {
            return Err(PublicHttpsError::Bound);
        }
        let models = models
            .iter()
            .map(|model| {
                let slug = model
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty() && s.len() <= 256)
                    .ok_or(PublicHttpsError::ModelSchema)?;
                let visibility = model
                    .get("visibility")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty() && s.len() <= 128)
                    .ok_or(PublicHttpsError::ModelSchema)?;
                Ok(DiscoveredModel {
                    slug: slug.into(),
                    visibility: visibility.into(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ModelResponse {
            models,
            response_sha256,
            response_bytes,
            raw_body: bytes,
        })
    }

    pub async fn revoke(
        &self,
        endpoint: &str,
        grant: &RevocationGrant,
    ) -> Result<bool, PublicHttpsError> {
        if !trusted_auth_path(endpoint, "/api/accounts/") || grant.endpoint != endpoint {
            return Err(PublicHttpsError::Policy);
        }
        let bytes = tokio::time::timeout(Duration::from_secs(5), async {
            let response = self
                .client
                .post(endpoint)
                .form(&[
                    ("token", grant.refresh_token()),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", grant.client_id.as_str()),
                ])
                .send()
                .await
                .map_err(|_| PublicHttpsError::Transport)?;
            read_bounded(response, 16 * 1024, ReadTimeout::Transport).await
        })
        .await
        .map_err(|_| PublicHttpsError::Transport)??;
        Ok(bytes.is_empty())
    }

    pub(crate) async fn send_response(
        &self,
        bearer: &BearerAccess,
        bytes: Vec<u8>,
    ) -> Result<Response, PublicHttpsError> {
        self.client
            .post(RESPONSES)
            .header(AUTHORIZATION, bearer.authorization_value())
            .header(ACCEPT, "text/event-stream")
            .header(CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .await
            .map_err(|_| PublicHttpsError::Transport)
    }
}

/// Preserve only reqwest's typed timeout signal; its message may include
/// request or peer details and must never reach public diagnostics.
fn classify_request_error(error: reqwest::Error) -> PublicHttpsError {
    if error.is_timeout() {
        PublicHttpsError::Deadline
    } else {
        PublicHttpsError::Transport
    }
}

pub(crate) fn trusted_auth_path(raw: &str, prefix: &str) -> bool {
    Url::parse(raw).ok().is_some_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("auth.openai.com")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path().starts_with(prefix)
            && url.path().len() <= 256
    })
}

enum ReadTimeout {
    Transport,
    Deadline,
}

async fn read_bounded(
    mut response: Response,
    limit: usize,
    timeout: ReadTimeout,
) -> Result<Vec<u8>, PublicHttpsError> {
    if response.status() != 200 {
        return Err(PublicHttpsError::HttpStatus(response.status().as_u16()));
    }
    if response
        .headers()
        .iter()
        .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
        .sum::<usize>()
        > MAX_HEADERS
    {
        return Err(PublicHttpsError::HeaderBound);
    }
    let mut bytes = Vec::new();
    {
        while let Some(chunk) = response.chunk().await.map_err(|error| match timeout {
            ReadTimeout::Transport => PublicHttpsError::Transport,
            ReadTimeout::Deadline => classify_request_error(error),
        })? {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(PublicHttpsError::BodyBound);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

async fn read_refresh_body(
    mut response: Response,
    limit: usize,
) -> Result<Vec<u8>, RefreshFailure> {
    if response
        .headers()
        .iter()
        .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
        .sum::<usize>()
        > MAX_HEADERS
    {
        return Err(RefreshFailure::Ambiguous);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| RefreshFailure::Ambiguous)?
    {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(RefreshFailure::Ambiguous);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
