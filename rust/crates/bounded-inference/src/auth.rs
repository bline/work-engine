//! Explicit, offline SIWC credential state. No method performs network I/O.
//! Endpoint responses and JWKS are caller-supplied; production endpoint use is
//! outside this crate's offline acceptance.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet, KeyAlgorithm, PublicKeyUse},
    Algorithm, DecodingKey, Validation,
};
use rustix::fs::{
    flock, fstat, fsync, openat, openat2, renameat, unlinkat, AtFlags, Dir, FlockOperation, Mode,
    OFlags, ResolveFlags, CWD,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Component, Path};
use std::time::{Duration, Instant};
use url::Url;

const MAX_STATE: usize = 256 * 1024;
const MAX_JWKS: usize = 64 * 1024;
const MAX_TOKEN: usize = 16 * 1024;
const MAX_CALLBACK: usize = 8 * 1024;
const MAX_REGISTRATIONS: usize = 32;
const PENDING_TTL: u64 = 600;
const REQUIRED_SCOPES: [&str; 6] = [
    "openid",
    "profile",
    "email",
    "offline_access",
    "resource.invoke",
    "chatgpt.tokens.use.direct",
];
const RESOURCE: &str = "https://api.openai.com/v1";

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AuthError {
    #[error("invalid auth input")]
    InvalidInput,
    #[error("unsafe credential store")]
    UnsafeStore,
    #[error("credential store operation failed")]
    Store,
    #[error("credential store exceeded its bound")]
    Bound,
    #[error("authorization attempt is pending")]
    Pending,
    #[error("authorization attempt was absent, stale, or consumed")]
    StaleAttempt,
    #[error("callback failed validation")]
    Callback,
    #[error("authorization was denied")]
    Denied,
    #[error("issued client identity is absent or inconsistent")]
    Client,
    #[error("signed identity failed validation")]
    Identity,
    #[error("required permission was not granted")]
    Scope,
    #[error("registration or credential revision changed")]
    StaleRevision,
    #[error("no selected usable credential")]
    Unavailable,
    #[error("refresh outcome is ambiguous and requires reauthorization")]
    AmbiguousRefresh,
    #[error("refresh client configuration rejected; repair configuration before another admitted attempt")]
    RefreshConfiguration,
    #[error("local credential state is absent")]
    InspectionAbsent,
    #[error("local credential state is corrupt")]
    InspectionCorrupt,
    #[error("local credential store lock is unavailable")]
    InspectionLockUnavailable,
}

#[derive(Clone, Serialize, Deserialize)]
struct Secret(String);
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}
impl Secret {
    fn new(s: String) -> Result<Self, AuthError> {
        if s.is_empty() || s.len() > MAX_TOKEN || s.bytes().any(|b| b < 32 || b == 127) {
            Err(AuthError::Bound)
        } else {
            Ok(Self(s))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issuer {
    issuer: String,
    authorize_endpoint: String,
    token_endpoint: String,
    revocation_endpoint: String,
    resource: String,
}
impl Issuer {
    /// Endpoint strings are explicit trusted discovery data. This constructor
    /// validates their shape; it does not fetch or authenticate discovery.
    pub fn new(
        issuer: &str,
        authorize_endpoint: &str,
        token_endpoint: &str,
        revocation_endpoint: &str,
    ) -> Result<Self, AuthError> {
        let iss = Url::parse(issuer).map_err(|_| AuthError::InvalidInput)?;
        let auth = Url::parse(authorize_endpoint).map_err(|_| AuthError::InvalidInput)?;
        let token = Url::parse(token_endpoint).map_err(|_| AuthError::InvalidInput)?;
        let revoke = Url::parse(revocation_endpoint).map_err(|_| AuthError::InvalidInput)?;
        if issuer.len() > 256
            || ![&iss, &auth, &token, &revoke].iter().all(|u| {
                u.scheme() == "https"
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.fragment().is_none()
                    && u.query().is_none()
                    && u.host_str() == iss.host_str()
            })
        {
            return Err(AuthError::InvalidInput);
        }
        Ok(Self {
            issuer: issuer.into(),
            authorize_endpoint: authorize_endpoint.into(),
            token_endpoint: token_endpoint.into(),
            revocation_endpoint: revocation_endpoint.into(),
            resource: RESOURCE.into(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRef {
    pub client_id: String,
    pub subject: String,
    pub credential_revision: u64,
}
/// Saved non-secret account choice. The embedded revision changes when
/// credentials change or sign-out completes; list again before a later action.
#[derive(Clone, PartialEq, Eq)]
pub struct SavedRegistration {
    pub registration: RegistrationRef,
    pub email: Option<String>,
    pub signed_in: bool,
}
impl fmt::Debug for SavedRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SavedRegistration")
            .field("registration", &self.registration)
            .field("email", &"[redacted]")
            .field("signed_in", &self.signed_in)
            .finish()
    }
}
impl fmt::Debug for RegistrationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegistrationRef")
            .field("credential_revision", &self.credential_revision)
            .finish_non_exhaustive()
    }
}

/// Only `authorize_url()` deliberately exposes the browser URL, which can contain
/// an ID-token hint. Never log that URL; Debug is redacted.
pub struct AuthStart {
    authorize_url: Secret,
    pub attempt_id: String,
}
impl AuthStart {
    pub fn authorize_url(&self) -> &str {
        &self.authorize_url.0
    }
}
impl fmt::Debug for AuthStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthStart")
            .field("attempt_id", &"[redacted]")
            .field("authorize_url", &"[redacted]")
            .finish()
    }
}

/// Caller sends these fields to its explicitly selected fake token peer.
/// The code and verifier cannot be serialized or printed via Debug.
pub struct CallbackExchange {
    attempt_id: String,
    client_id: String,
    redirect_uri: String,
    resource: String,
    token_endpoint: String,
    code: Secret,
    verifier: Secret,
}
impl CallbackExchange {
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
    pub fn resource(&self) -> &str {
        &self.resource
    }
    pub fn token_endpoint(&self) -> &str {
        &self.token_endpoint
    }
    pub fn code(&self) -> &str {
        &self.code.0
    }
    pub fn code_verifier(&self) -> &str {
        &self.verifier.0
    }
}
impl fmt::Debug for CallbackExchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CallbackExchange([redacted])")
    }
}

#[derive(Deserialize)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    id_token: String,
    token_type: String,
    expires_in: u64,
    scope: String,
    #[serde(default)]
    earliest_refresh_at: Option<serde_json::Value>,
}
impl fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenResponse([redacted])")
    }
}
impl TokenResponse {
    pub fn from_json(bytes: &[u8]) -> Result<Self, AuthError> {
        if bytes.len() > MAX_STATE {
            return Err(AuthError::Bound);
        }
        serde_json::from_slice(bytes).map_err(|_| AuthError::InvalidInput)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness {
    SignedOut,
    Ready(RegistrationRef),
    Expired(RegistrationRef),
    RefreshAmbiguous(RegistrationRef),
    ReauthorizationRequired(RegistrationRef),
}

/// A per-operation snapshot. The caller must check expires_at immediately
/// before sending; it is capped to 30 seconds and carries the credential revision.
pub struct BearerAccess {
    pub registration: RegistrationRef,
    pub expires_at: u64,
    token: Secret,
}
impl BearerAccess {
    pub fn authorization_value(&self) -> String {
        format!("Bearer {}", self.token.0)
    }
}
impl fmt::Debug for BearerAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BearerAccess")
            .field("registration", &self.registration)
            .field("expires_at", &self.expires_at)
            .field("token", &"[redacted]")
            .finish()
    }
}

pub struct RefreshGrant {
    pub registration: RegistrationRef,
    pub client_id: String,
    pub resource: String,
    refresh_token: Secret,
}
impl RefreshGrant {
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token.0
    }
}
impl fmt::Debug for RefreshGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RefreshGrant([redacted])")
    }
}

pub struct RevocationGrant {
    pub client_id: String,
    pub endpoint: String,
    pub token_type_hint: &'static str,
    refresh_token: Secret,
}
impl RevocationGrant {
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token.0
    }
}
impl fmt::Debug for RevocationGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RevocationGrant([redacted])")
    }
}

/// Temporary is only for a failure known not to have rotated the remote token.
/// Configuration is a complete invalid-client rejection, not a token verdict.
/// Any sent request with an unknown outcome must be Ambiguous.
pub enum RefreshFailure {
    Temporary,
    Ambiguous,
    Terminal,
    Configuration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationOutcome {
    Confirmed,
    Unconfirmed,
    NoRenewableSession,
}

#[derive(Serialize, Deserialize, Clone)]
struct Credential {
    access: Secret,
    refresh: Secret,
    id: Secret,
    scopes: Vec<String>,
    expires_at: u64,
    earliest_refresh_at: Option<serde_json::Value>,
}
#[derive(Serialize, Deserialize, Clone)]
struct Registration {
    client_id: String,
    subject: String,
    email: Option<String>,
    revision: u64,
    credential: Option<Credential>,
    refresh_state: RefreshState,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
enum RefreshState {
    Idle,
    InFlight,
    Ambiguous,
    Terminal,
    Revoking,
}
#[derive(Serialize, Deserialize, Clone)]
struct Pending {
    attempt_id: String,
    state: Secret,
    nonce: Secret,
    verifier: Secret,
    redirect_uri: String,
    client_id: Option<String>,
    subject: Option<String>,
    selected_revision: Option<u64>,
    selection_revision: u64,
    issuer: Issuer,
    issued_client_id: Option<String>,
    code: Option<Secret>,
    expires_at: u64,
}
#[derive(Serialize, Deserialize, Clone)]
struct State {
    version: u8,
    host_id: String,
    issuer: Issuer,
    revision: u64,
    selected_client_id: Option<String>,
    registrations: Vec<Registration>,
    pending: Option<Pending>,
    #[serde(default)]
    unverified_issued_client_id: Option<String>,
}
impl State {
    fn new(host_id: String, issuer: Issuer) -> Self {
        Self {
            version: 2,
            host_id,
            issuer,
            revision: 0,
            selected_client_id: None,
            registrations: Vec::new(),
            pending: None,
            unverified_issued_client_id: None,
        }
    }
}

pub struct AuthStore {
    root: File,
    issuer: Issuer,
    #[cfg(feature = "auth-test-faults")]
    fault: std::sync::Mutex<Option<StoreFault>>,
}
/// Test-only injection at actual atomic publication operations. This type and
/// its setter are absent from the default build.
#[cfg(feature = "auth-test-faults")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreFault {
    Write,
    FileSync,
    Rename,
    DirectorySync,
    ExitAfterWrite,
    ExitAfterFileSync,
    ExitAfterRename,
    ExitAfterDirectorySync,
}
impl AuthStore {
    #[cfg(feature = "auth-test-faults")]
    pub fn set_store_fault(&self, fault: StoreFault) {
        *self.fault.lock().expect("test fault lock") = Some(fault);
    }
    #[cfg(feature = "auth-test-faults")]
    fn fault_at(&self, stage: StoreFault) -> Result<(), AuthError> {
        let mut selected = self.fault.lock().expect("test fault lock");
        if *selected == Some(stage) {
            *selected = None;
            if matches!(
                stage,
                StoreFault::ExitAfterWrite
                    | StoreFault::ExitAfterFileSync
                    | StoreFault::ExitAfterRename
                    | StoreFault::ExitAfterDirectorySync
            ) {
                std::process::exit(0);
            }
            return Err(AuthError::Store);
        }
        Ok(())
    }
    /// The caller supplies a dedicated absolute path; no home or environment
    /// search occurs. On Linux every root path component is opened without symlinks.
    pub fn open(root: &Path, host_id: &str, issuer: Issuer) -> Result<Self, AuthError> {
        if !root.is_absolute()
            || root
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || root.as_os_str().len() > 4096
            || !valid_host_id(host_id)
        {
            return Err(AuthError::InvalidInput);
        }
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        let created = match builder.create(root) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(_) => return Err(AuthError::Store),
        };
        let fd = openat2(
            CWD,
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS,
        )
        .map_err(|_| AuthError::UnsafeStore)?;
        check_fd(&fd, true)?;
        let store = Self {
            root: File::from(fd),
            issuer,
            #[cfg(feature = "auth-test-faults")]
            fault: std::sync::Mutex::new(None),
        };
        store.transaction(
            |s| {
                if s.host_id != host_id || s.issuer != store.issuer {
                    return Err(AuthError::UnsafeStore);
                }
                Ok(((), created))
            },
            if created { host_id } else { "" },
        )?;
        Ok(store)
    }

    /// Inspect an existing store without initialization, cleanup, refresh, or writes.
    /// The shared lock gives a coherent snapshot against normal store writers.
    pub fn inspect_readiness(
        root: &Path,
        host_id: &str,
        issuer: Issuer,
        now: u64,
    ) -> Result<Readiness, AuthError> {
        Self::inspect_state(root, host_id, issuer).map(|state| readiness_from_state(&state, now))
    }

    /// Observe only the selected, idle registration with a locally present,
    /// structurally valid refresh credential. This does not establish remote
    /// token usability and never changes the store.
    pub fn inspect_renewal_target(
        root: &Path,
        host_id: &str,
        issuer: Issuer,
    ) -> Result<RegistrationRef, AuthError> {
        let state = Self::inspect_state(root, host_id, issuer)?;
        renewal_target_from_state(&state)
    }

    fn inspect_state(root: &Path, host_id: &str, issuer: Issuer) -> Result<State, AuthError> {
        if !root.is_absolute()
            || root
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || root.as_os_str().len() > 4096
            || !valid_host_id(host_id)
        {
            return Err(AuthError::InvalidInput);
        }
        let fd = match openat2(
            CWD,
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS,
        ) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Err(AuthError::InspectionAbsent),
            Err(_) => return Err(AuthError::UnsafeStore),
        };
        check_fd(&fd, true)?;
        let store = Self {
            root: File::from(fd),
            issuer,
            #[cfg(feature = "auth-test-faults")]
            fault: std::sync::Mutex::new(None),
        };
        let lock = match openat(
            &store.root,
            ".lock",
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Err(AuthError::InspectionLockUnavailable),
            Err(_) => return Err(AuthError::UnsafeStore),
        };
        check_fd(&lock, false)?;
        let lock = File::from(lock);
        flock(&lock, FlockOperation::NonBlockingLockShared)
            .map_err(|_| AuthError::InspectionLockUnavailable)?;
        match openat(
            &store.root,
            "state.json",
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => {
                check_fd(&fd, false)?;
            }
            Err(rustix::io::Errno::NOENT) => return Err(AuthError::InspectionAbsent),
            Err(_) => return Err(AuthError::UnsafeStore),
        }
        let state = store.read_state(&lock, "").map_err(|error| match error {
            AuthError::UnsafeStore => AuthError::UnsafeStore,
            _ => AuthError::InspectionCorrupt,
        })?;
        if state.host_id != host_id {
            return Err(AuthError::UnsafeStore);
        }
        Ok(state)
    }

    pub fn begin_registration(
        &self,
        app_name: &str,
        redirect_uri: &str,
        now: u64,
    ) -> Result<AuthStart, AuthError> {
        if app_name.is_empty()
            || app_name.len() > 100
            || app_name.bytes().any(|b| b < 32 || b == 127)
        {
            return Err(AuthError::InvalidInput);
        }
        self.begin(None, Some(app_name), redirect_uri, now)
    }
    /// After an invalid_grant exchange, retry with the issued client from the
    /// prior callback. It remains unverified and never becomes a credential.
    pub fn begin_issued_retry(&self, redirect_uri: &str, now: u64) -> Result<AuthStart, AuthError> {
        self.begin(None, None, redirect_uri, now)
    }
    pub fn begin_returning(
        &self,
        selected: &RegistrationRef,
        redirect_uri: &str,
        now: u64,
    ) -> Result<AuthStart, AuthError> {
        self.begin(Some(selected), None, redirect_uri, now)
    }
    fn begin(
        &self,
        selected: Option<&RegistrationRef>,
        app_name: Option<&str>,
        redirect_uri: &str,
        now: u64,
    ) -> Result<AuthStart, AuthError> {
        check_callback_base(redirect_uri)?;
        let state = random_value()?;
        let nonce = random_value()?;
        let verifier = random_value()?;
        let attempt_id = random_value()?;
        let redirect = redirect_uri.to_string();
        let issuer = self.issuer.clone();
        self.transaction(
            |s| {
                if s.pending.as_ref().is_some_and(|p| p.expires_at > now) {
                    return Err(AuthError::Pending);
                }
                let reg = if let Some(r) = selected {
                    Some(find_reg(s, r)?.clone())
                } else {
                    None
                };
                let unverified = if selected.is_none() && app_name.is_none() {
                    Some(
                        s.unverified_issued_client_id
                            .clone()
                            .ok_or(AuthError::Client)?,
                    )
                } else {
                    None
                };
                let mut url =
                    Url::parse(&issuer.authorize_endpoint).map_err(|_| AuthError::InvalidInput)?;
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("response_type", "code")
                        .append_pair(
                            "client_id",
                            reg.as_ref()
                                .map(|r| r.client_id.as_str())
                                .or(unverified.as_deref())
                                .unwrap_or("dynamic_agent_client"),
                        )
                        .append_pair("ext_agent_host_id", &s.host_id)
                        .append_pair("redirect_uri", &redirect)
                        .append_pair("scope", &REQUIRED_SCOPES.join(" "))
                        .append_pair("resource", &issuer.resource)
                        .append_pair("state", &state)
                        .append_pair("nonce", &nonce)
                        .append_pair("code_challenge_method", "S256")
                        .append_pair(
                            "code_challenge",
                            &URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
                        );
                    if let Some(name) = app_name {
                        q.append_pair("agent_name_hint", name);
                    }
                    if let Some(r) = &reg {
                        if let Some(c) = &r.credential {
                            q.append_pair("id_token_hint", &c.id.0);
                        }
                        if let Some(email) = &r.email {
                            q.append_pair("login_hint", email);
                        }
                    }
                }
                s.pending = Some(Pending {
                    attempt_id: attempt_id.clone(),
                    state: Secret::new(state.clone())?,
                    nonce: Secret::new(nonce.clone())?,
                    verifier: Secret::new(verifier.clone())?,
                    redirect_uri: redirect.clone(),
                    client_id: reg.as_ref().map(|r| r.client_id.clone()).or(unverified),
                    subject: reg.as_ref().map(|r| r.subject.clone()),
                    selected_revision: reg.as_ref().map(|r| r.revision),
                    selection_revision: s.revision,
                    issuer: issuer.clone(),
                    issued_client_id: None,
                    code: None,
                    expires_at: now.saturating_add(PENDING_TTL),
                });
                Ok((
                    AuthStart {
                        authorize_url: Secret::new(url.to_string())?,
                        attempt_id: attempt_id.clone(),
                    },
                    true,
                ))
            },
            "",
        )
    }

    /// Validates the exact loopback callback and consumes the authorization
    /// state before any code exchange. Replaying it cannot return the code twice.
    pub fn accept_callback(
        &self,
        callback_url: &str,
        now: u64,
    ) -> Result<CallbackExchange, AuthError> {
        if callback_url.len() > MAX_CALLBACK {
            return Err(AuthError::Bound);
        }
        let callback = Url::parse(callback_url).map_err(|_| AuthError::Callback)?;
        if callback.fragment().is_some()
            || callback.username() != ""
            || callback.password().is_some()
        {
            return Err(AuthError::Callback);
        }
        let mut fields = std::collections::HashMap::<String, String>::new();
        for (k, v) in callback.query_pairs() {
            if fields.insert(k.to_string(), v.to_string()).is_some() {
                return Err(AuthError::Callback);
            }
        }
        let issuer = self.issuer.clone();
        self.transaction(
            |s| {
                let pending = s.pending.as_mut().ok_or(AuthError::StaleAttempt)?;
                if pending.issuer != self.issuer {
                    return Err(AuthError::UnsafeStore);
                }
                if pending.expires_at <= now || pending.code.is_some() {
                    return Err(AuthError::StaleAttempt);
                }
                let mut expected =
                    Url::parse(&pending.redirect_uri).map_err(|_| AuthError::Store)?;
                expected.set_query(None);
                let mut actual = callback.clone();
                actual.set_query(None);
                if actual != expected
                    || fields.get("state").map(String::as_str) != Some(pending.state.0.as_str())
                {
                    return Err(AuthError::Callback);
                }
                if fields.contains_key("error") {
                    s.pending = None;
                    return Ok((Err(AuthError::Denied), true));
                }
                let code = Secret::new(fields.get("code").cloned().ok_or(AuthError::Callback)?)?;
                let issued = match (&pending.client_id, fields.get("client_id")) {
                    (None, Some(c)) if valid_id(c) && c != "dynamic_agent_client" => c.clone(),
                    (Some(known), None) => known.clone(),
                    (Some(known), Some(c)) if c == known => known.clone(),
                    _ => return Err(AuthError::Client),
                };
                pending.issued_client_id = Some(issued.clone());
                pending.code = Some(code.clone());
                pending.state = Secret::new("consumed".into())?;
                Ok((
                    Ok(CallbackExchange {
                        attempt_id: pending.attempt_id.clone(),
                        client_id: issued,
                        redirect_uri: pending.redirect_uri.clone(),
                        resource: issuer.resource,
                        token_endpoint: issuer.token_endpoint,
                        code,
                        verifier: pending.verifier.clone(),
                    }),
                    true,
                ))
            },
            "",
        )?
    }

    pub fn invalid_grant(&self, exchange: &CallbackExchange) -> Result<(), AuthError> {
        self.transaction(
            |s| {
                let p = s.pending.as_ref().ok_or(AuthError::StaleAttempt)?;
                if p.attempt_id != exchange.attempt_id
                    || p.issued_client_id.as_deref() != Some(&exchange.client_id)
                    || p.code.as_ref().map(|c| &c.0) != Some(&exchange.code.0)
                {
                    return Err(AuthError::StaleAttempt);
                }
                if p.subject.is_none() {
                    s.unverified_issued_client_id = Some(exchange.client_id.clone());
                }
                s.pending = None;
                Ok(((), true))
            },
            "",
        )
    }

    pub fn finish_exchange(
        &self,
        exchange: &CallbackExchange,
        response: TokenResponse,
        jwks: &str,
        now: u64,
    ) -> Result<RegistrationRef, AuthError> {
        self.transaction(
            |s| {
                let p = s.pending.as_ref().ok_or(AuthError::StaleAttempt)?;
                if p.expires_at <= now
                    || p.attempt_id != exchange.attempt_id
                    || p.issued_client_id.as_deref() != Some(exchange.client_id.as_str())
                    || p.code.as_ref().map(|v| &v.0) != Some(&exchange.code.0)
                    || p.redirect_uri != exchange.redirect_uri
                    || exchange.resource != self.issuer.resource
                    || exchange.token_endpoint != self.issuer.token_endpoint
                    || p.selection_revision != s.revision
                    || p.issuer != self.issuer
                {
                    return Err(AuthError::StaleAttempt);
                }
                let cred = credential(response, now)?;
                let identity = validate_id(
                    &cred.id.0,
                    jwks,
                    &self.issuer.issuer,
                    &exchange.client_id,
                    Some(&p.nonce.0),
                    now,
                )?;
                if let Some(sub) = &p.subject {
                    if sub != &identity.sub {
                        return Err(AuthError::Identity);
                    }
                }
                if let Some(old) = s
                    .registrations
                    .iter()
                    .find(|r| r.client_id == exchange.client_id)
                {
                    if old.subject != identity.sub {
                        return Err(AuthError::Identity);
                    }
                }
                if let Some(rev) = p.selected_revision {
                    let old = s
                        .registrations
                        .iter()
                        .find(|r| r.client_id == exchange.client_id)
                        .ok_or(AuthError::StaleRevision)?;
                    if old.revision != rev {
                        return Err(AuthError::StaleRevision);
                    }
                }
                let new_rev = s.revision.checked_add(1).ok_or(AuthError::Bound)?;
                s.revision = new_rev;
                if let Some(old) = s
                    .registrations
                    .iter_mut()
                    .find(|r| r.client_id == exchange.client_id)
                {
                    old.revision = new_rev;
                    old.credential = Some(cred);
                    old.refresh_state = RefreshState::Idle;
                    old.email = identity.email;
                } else {
                    if s.registrations.len() >= MAX_REGISTRATIONS {
                        return Err(AuthError::Bound);
                    }
                    s.registrations.push(Registration {
                        client_id: exchange.client_id.clone(),
                        subject: identity.sub.clone(),
                        email: identity.email,
                        revision: new_rev,
                        credential: Some(cred),
                        refresh_state: RefreshState::Idle,
                    });
                }
                s.selected_client_id = Some(exchange.client_id.clone());
                s.pending = None;
                s.unverified_issued_client_id = None;
                Ok((
                    RegistrationRef {
                        client_id: exchange.client_id.clone(),
                        subject: identity.sub,
                        credential_revision: new_rev,
                    },
                    true,
                ))
            },
            "",
        )
    }

    pub fn readiness(&self, now: u64) -> Result<Readiness, AuthError> {
        self.transaction(
            |s| {
                let view = readiness_from_state(s, now);
                Ok((view, false))
            },
            "",
        )
    }
    pub fn saved_registrations(&self) -> Result<Vec<SavedRegistration>, AuthError> {
        self.transaction(
            |s| {
                Ok((
                    s.registrations
                        .iter()
                        .map(|r| SavedRegistration {
                            registration: reg_ref(r),
                            email: r.email.clone(),
                            signed_in: r.credential.is_some()
                                && r.refresh_state == RefreshState::Idle,
                        })
                        .collect(),
                    false,
                ))
            },
            "",
        )
    }
    pub fn bearer(&self, reference: &RegistrationRef, now: u64) -> Result<BearerAccess, AuthError> {
        self.transaction(
            |s| {
                if s.selected_client_id.as_deref() != Some(&reference.client_id) {
                    return Err(AuthError::StaleRevision);
                }
                let r = find_reg(s, reference)?;
                if r.refresh_state != RefreshState::Idle {
                    return Err(AuthError::Unavailable);
                }
                let c = r.credential.as_ref().ok_or(AuthError::Unavailable)?;
                if c.expires_at <= now {
                    return Err(AuthError::Unavailable);
                }
                Ok((
                    BearerAccess {
                        registration: reg_ref(r),
                        expires_at: c.expires_at.min(now.saturating_add(30)),
                        token: c.access.clone(),
                    },
                    false,
                ))
            },
            "",
        )
    }
    pub fn switch(&self, reference: &RegistrationRef) -> Result<(), AuthError> {
        self.transaction(
            |s| {
                find_reg(s, reference)?;
                s.selected_client_id = Some(reference.client_id.clone());
                s.revision = s.revision.checked_add(1).ok_or(AuthError::Bound)?;
                Ok(((), true))
            },
            "",
        )
    }

    /// The closure is invoked exactly once while a cross-process exclusive lock
    /// is held. An in-flight marker is durable first. A panic or ambiguous loss
    /// leaves the session unusable until explicit reauthorization.
    pub fn refresh_with<F>(
        &self,
        reference: &RegistrationRef,
        now: u64,
        jwks: &str,
        exchange: F,
    ) -> Result<RegistrationRef, AuthError>
    where
        F: FnOnce(&RefreshGrant) -> Result<TokenResponse, RefreshFailure>,
    {
        let guard = self.lock()?;
        let mut s = self.read_state(&guard, "")?;
        if s.selected_client_id.as_deref() != Some(&reference.client_id) {
            return Err(AuthError::StaleRevision);
        }
        let r = find_reg_mut(&mut s, reference)?;
        if r.refresh_state != RefreshState::Idle {
            return Err(AuthError::AmbiguousRefresh);
        }
        let c = r.credential.as_ref().ok_or(AuthError::Unavailable)?;
        if !valid_refresh_secret(&c.refresh) {
            return Err(AuthError::Unavailable);
        }
        let grant = RefreshGrant {
            registration: reg_ref(r),
            client_id: r.client_id.clone(),
            resource: self.issuer.resource.clone(),
            refresh_token: c.refresh.clone(),
        };
        r.refresh_state = RefreshState::InFlight;
        self.write_state(&s)?;
        let result = exchange(&grant);
        let next_rev = s.revision.checked_add(1).ok_or(AuthError::Bound)?;
        let r = find_reg_mut(&mut s, reference)?;
        match result {
            Ok(response) => {
                let c = credential(response, now)?;
                let identity =
                    validate_id(&c.id.0, jwks, &self.issuer.issuer, &r.client_id, None, now)?;
                if identity.sub != r.subject {
                    return Err(AuthError::Identity);
                }
                r.credential = Some(c);
                r.refresh_state = RefreshState::Idle;
                r.revision = next_rev;
                let out = reg_ref(r);
                s.revision = next_rev;
                self.write_state(&s)?;
                Ok(out)
            }
            Err(RefreshFailure::Temporary) => {
                r.refresh_state = RefreshState::Idle;
                self.write_state(&s)?;
                Err(AuthError::Unavailable)
            }
            Err(RefreshFailure::Configuration) => {
                r.refresh_state = RefreshState::Idle;
                self.write_state(&s)?;
                Err(AuthError::RefreshConfiguration)
            }
            Err(RefreshFailure::Terminal) => {
                r.refresh_state = RefreshState::Terminal;
                self.write_state(&s)?;
                Err(AuthError::Unavailable)
            }
            Err(RefreshFailure::Ambiguous) => {
                r.refresh_state = RefreshState::Ambiguous;
                self.write_state(&s)?;
                Err(AuthError::AmbiguousRefresh)
            }
        }
    }

    /// Commit local disable before the caller attempts optional remote revocation.
    /// The returned grant remains protected in process memory only.
    pub fn sign_out_local(
        &self,
        reference: &RegistrationRef,
    ) -> Result<Option<RevocationGrant>, AuthError> {
        let guard = self.lock()?;
        let mut s = self.read_state(&guard, "")?;
        if s.selected_client_id.as_deref() != Some(&reference.client_id) {
            return Err(AuthError::StaleRevision);
        }
        let next_rev = s.revision.checked_add(1).ok_or(AuthError::Bound)?;
        let r = find_reg_mut(&mut s, reference)?;
        let grant = r.credential.as_ref().map(|c| RevocationGrant {
            client_id: r.client_id.clone(),
            endpoint: self.issuer.revocation_endpoint.clone(),
            token_type_hint: "refresh_token",
            refresh_token: c.refresh.clone(),
        });
        r.credential = None;
        r.refresh_state = RefreshState::Idle;
        r.revision = next_rev;
        s.selected_client_id = None;
        s.pending = None;
        s.revision = next_rev;
        self.write_state(&s)?;
        Ok(grant)
    }

    /// The supplied closure performs fake revocation, if a token exists. Local
    /// sign-out is committed even when remote revocation cannot be confirmed.
    pub fn sign_out_with<F>(
        &self,
        reference: &RegistrationRef,
        revoke: F,
    ) -> Result<RevocationOutcome, AuthError>
    where
        F: FnOnce(&RevocationGrant) -> bool,
    {
        let guard = self.lock()?;
        let mut s = self.read_state(&guard, "")?;
        if s.selected_client_id.as_deref() != Some(&reference.client_id) {
            return Err(AuthError::StaleRevision);
        }
        let next_rev = s.revision.checked_add(1).ok_or(AuthError::Bound)?;
        let r = find_reg_mut(&mut s, reference)?;
        r.refresh_state = RefreshState::Revoking;
        self.write_state(&s)?;
        let r = find_reg_mut(&mut s, reference)?;
        let outcome = match &r.credential {
            Some(c) => {
                let grant = RevocationGrant {
                    client_id: r.client_id.clone(),
                    endpoint: self.issuer.revocation_endpoint.clone(),
                    token_type_hint: "refresh_token",
                    refresh_token: c.refresh.clone(),
                };
                if revoke(&grant) {
                    RevocationOutcome::Confirmed
                } else {
                    RevocationOutcome::Unconfirmed
                }
            }
            None => RevocationOutcome::NoRenewableSession,
        };
        r.credential = None;
        r.refresh_state = RefreshState::Idle;
        r.revision = next_rev;
        s.selected_client_id = None;
        s.pending = None;
        s.revision = next_rev;
        self.write_state(&s)?;
        Ok(outcome)
    }

    fn lock(&self) -> Result<File, AuthError> {
        let fd = openat(
            &self.root,
            ".lock",
            OFlags::CREATE | OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::from_bits_retain(0o600),
        )
        .map_err(|_| AuthError::UnsafeStore)?;
        check_fd(&fd, false)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match flock(&fd, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => break,
                Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => return Err(AuthError::Store),
            }
        }
        self.clean_orphans()?;
        Ok(File::from(fd))
    }
    fn clean_orphans(&self) -> Result<(), AuthError> {
        let mut dir = Dir::read_from(&self.root).map_err(|_| AuthError::Store)?;
        let mut removed = false;
        for entry in &mut dir {
            let entry = entry.map_err(|_| AuthError::Store)?;
            let name = entry.file_name().to_bytes();
            let suffix = name.strip_prefix(b".state-");
            if let Some(suffix) = suffix {
                if suffix.len() == 43
                    && suffix
                        .iter()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
                {
                    unlinkat(&self.root, entry.file_name(), AtFlags::empty())
                        .map_err(|_| AuthError::Store)?;
                    removed = true;
                }
            }
        }
        if removed {
            fsync(&self.root).map_err(|_| AuthError::Store)?;
        }
        Ok(())
    }
    fn transaction<T>(
        &self,
        f: impl FnOnce(&mut State) -> Result<(T, bool), AuthError>,
        host_id: &str,
    ) -> Result<T, AuthError> {
        let guard = self.lock()?;
        let mut s = self.read_state(&guard, host_id)?;
        let (out, changed) = f(&mut s)?;
        if changed {
            self.write_state(&s)?;
        }
        Ok(out)
    }
    fn read_state(&self, _guard: &File, host_id: &str) -> Result<State, AuthError> {
        let fd = match openat(
            &self.root,
            "state.json",
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(e) if e == rustix::io::Errno::NOENT && !host_id.is_empty() => {
                return Ok(State::new(host_id.into(), self.issuer.clone()))
            }
            Err(_) => return Err(AuthError::UnsafeStore),
        };
        check_fd(&fd, false)?;
        let mut file = File::from(fd);
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take((MAX_STATE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| AuthError::Store)?;
        if bytes.len() > MAX_STATE {
            return Err(AuthError::Bound);
        }
        let state: State = serde_json::from_slice(&bytes).map_err(|_| AuthError::Store)?;
        if state.issuer != self.issuer {
            return Err(AuthError::UnsafeStore);
        }
        let ids: std::collections::HashSet<_> =
            state.registrations.iter().map(|r| &r.client_id).collect();
        if state.version != 2
            || !valid_host_id(&state.host_id)
            || state.registrations.len() > MAX_REGISTRATIONS
            || ids.len() != state.registrations.len()
            || state
                .selected_client_id
                .as_ref()
                .is_some_and(|id| !ids.contains(id))
        {
            return Err(AuthError::Store);
        }
        Ok(state)
    }
    fn write_state(&self, s: &State) -> Result<(), AuthError> {
        let bytes = serde_json::to_vec(s).map_err(|_| AuthError::Store)?;
        if bytes.len() > MAX_STATE {
            return Err(AuthError::Bound);
        }
        let temp = format!(".state-{}", random_value()?);
        let fd = openat(
            &self.root,
            temp.as_str(),
            OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::from_bits_retain(0o600),
        )
        .map_err(|_| AuthError::Store)?;
        let mut file = File::from(fd);
        let result = (|| {
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::Write)?;
            file.write_all(&bytes).map_err(|_| AuthError::Store)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::ExitAfterWrite)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::FileSync)?;
            file.sync_all().map_err(|_| AuthError::Store)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::ExitAfterFileSync)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::Rename)?;
            renameat(&self.root, temp.as_str(), &self.root, "state.json")
                .map_err(|_| AuthError::Store)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::ExitAfterRename)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::DirectorySync)?;
            fsync(&self.root).map_err(|_| AuthError::Store)?;
            #[cfg(feature = "auth-test-faults")]
            self.fault_at(StoreFault::ExitAfterDirectorySync)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlinkat(&self.root, temp.as_str(), AtFlags::empty());
            let _ = fsync(&self.root);
        }
        result
    }
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
fn valid_host_id(s: &str) -> bool {
    if valid_id(s) {
        return true;
    }
    if s.len() > 256 {
        return false;
    }
    let Some(value) = s
        .strip_prefix("urn:uuid:")
        .or_else(|| s.strip_prefix("urn:ietf:params:oauth:jwk-thumbprint:"))
        .or_else(|| s.strip_prefix("did:key:"))
    else {
        return false;
    };
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b':'))
}
fn random_value() -> Result<String, AuthError> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|_| AuthError::Store)?;
    Ok(URL_SAFE_NO_PAD.encode(b))
}
fn check_callback_base(raw: &str) -> Result<(), AuthError> {
    let u = Url::parse(raw).map_err(|_| AuthError::InvalidInput)?;
    if raw.len() > 512
        || u.scheme() != "http"
        || u.host_str() != Some("127.0.0.1")
        || u.port().is_none()
        || u.path() != "/auth/callback"
        || u.query().is_some()
        || u.fragment().is_some()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        Err(AuthError::InvalidInput)
    } else {
        Ok(())
    }
}
fn check_fd(fd: &impl std::os::fd::AsFd, dir: bool) -> Result<(), AuthError> {
    let st = fstat(fd).map_err(|_| AuthError::Store)?;
    let ty = st.st_mode & 0o170000;
    if st.st_uid != rustix::process::getuid().as_raw()
        || st.st_mode & 0o077 != 0
        || (dir && st.st_mode & 0o700 != 0o700)
        || (if dir { ty != 0o040000 } else { ty != 0o100000 })
        || (!dir && st.st_nlink != 1)
    {
        Err(AuthError::UnsafeStore)
    } else {
        Ok(())
    }
}
fn readiness_from_state(s: &State, now: u64) -> Readiness {
    let registration = match &s.selected_client_id {
        Some(id) => s.registrations.iter().find(|r| &r.client_id == id),
        None => None,
    };
    match registration {
        None => Readiness::SignedOut,
        Some(r) => {
            let reference = reg_ref(r);
            match r.refresh_state {
                RefreshState::Ambiguous | RefreshState::InFlight => {
                    Readiness::RefreshAmbiguous(reference)
                }
                RefreshState::Terminal | RefreshState::Revoking => {
                    Readiness::ReauthorizationRequired(reference)
                }
                RefreshState::Idle => match &r.credential {
                    Some(c) if c.expires_at > now => Readiness::Ready(reference),
                    _ => Readiness::Expired(reference),
                },
            }
        }
    }
}

fn valid_refresh_secret(refresh: &Secret) -> bool {
    !refresh.0.is_empty()
        && refresh.0.len() <= MAX_TOKEN
        && !refresh.0.bytes().any(|b| b < 32 || b == 127)
}

fn renewal_target_from_state(s: &State) -> Result<RegistrationRef, AuthError> {
    let selected = s
        .selected_client_id
        .as_ref()
        .ok_or(AuthError::Unavailable)?;
    let registration = s
        .registrations
        .iter()
        .find(|r| &r.client_id == selected)
        .ok_or(AuthError::Unavailable)?;
    if registration.refresh_state != RefreshState::Idle
        || !registration
            .credential
            .as_ref()
            .is_some_and(|credential| valid_refresh_secret(&credential.refresh))
    {
        return Err(AuthError::Unavailable);
    }
    Ok(reg_ref(registration))
}

fn reg_ref(r: &Registration) -> RegistrationRef {
    RegistrationRef {
        client_id: r.client_id.clone(),
        subject: r.subject.clone(),
        credential_revision: r.revision,
    }
}
fn find_reg<'a>(s: &'a State, rf: &RegistrationRef) -> Result<&'a Registration, AuthError> {
    s.registrations
        .iter()
        .find(|r| {
            r.client_id == rf.client_id
                && r.subject == rf.subject
                && r.revision == rf.credential_revision
        })
        .ok_or(AuthError::StaleRevision)
}
fn find_reg_mut<'a>(
    s: &'a mut State,
    rf: &RegistrationRef,
) -> Result<&'a mut Registration, AuthError> {
    s.registrations
        .iter_mut()
        .find(|r| {
            r.client_id == rf.client_id
                && r.subject == rf.subject
                && r.revision == rf.credential_revision
        })
        .ok_or(AuthError::StaleRevision)
}
fn credential(t: TokenResponse, now: u64) -> Result<Credential, AuthError> {
    if t.token_type != "Bearer" || t.expires_in == 0 || t.expires_in > 86400 || t.scope.len() > 2048
    {
        return Err(AuthError::InvalidInput);
    }
    let scopes: Vec<String> = t.scope.split_whitespace().map(str::to_owned).collect();
    if !REQUIRED_SCOPES
        .iter()
        .all(|s| scopes.iter().any(|v| v == s))
    {
        return Err(AuthError::Scope);
    }
    if let Some(v) = &t.earliest_refresh_at {
        if v.to_string().len() > 256 {
            return Err(AuthError::Bound);
        }
    }
    Ok(Credential {
        access: Secret::new(t.access_token)?,
        refresh: Secret::new(t.refresh_token)?,
        id: Secret::new(t.id_token)?,
        scopes,
        expires_at: now.checked_add(t.expires_in).ok_or(AuthError::Bound)?,
        earliest_refresh_at: t.earliest_refresh_at,
    })
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}
impl Audience {
    fn includes(&self, s: &str) -> bool {
        match self {
            Self::One(v) => v == s,
            Self::Many(v) => v.len() == 1 && v[0] == s,
        }
    }
}
#[derive(Deserialize)]
struct IdClaims {
    iss: String,
    aud: Audience,
    sub: String,
    exp: u64,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    iat: Option<u64>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    email: Option<String>,
}
struct Identity {
    sub: String,
    email: Option<String>,
}
fn validate_id(
    token: &str,
    jwks: &str,
    issuer: &str,
    client_id: &str,
    nonce: Option<&str>,
    now: u64,
) -> Result<Identity, AuthError> {
    if token.len() > MAX_TOKEN || jwks.len() > MAX_JWKS {
        return Err(AuthError::Bound);
    }
    let header = decode_header(token).map_err(|_| AuthError::Identity)?;
    if header.alg != Algorithm::RS256 {
        return Err(AuthError::Identity);
    }
    let kid = header
        .kid
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or(AuthError::Identity)?;
    let set: JwkSet = serde_json::from_str(jwks).map_err(|_| AuthError::Identity)?;
    if set.keys.len() > 32 {
        return Err(AuthError::Bound);
    }
    let mut matches = set
        .keys
        .iter()
        .filter(|k| k.common.key_id.as_deref() == Some(kid));
    let key = matches.next().ok_or(AuthError::Identity)?;
    if matches.next().is_some() {
        return Err(AuthError::Identity);
    }
    if !matches!(key.algorithm, AlgorithmParameters::RSA(_))
        || key.common.key_algorithm != Some(KeyAlgorithm::RS256)
        || key.common.public_key_use.as_ref() != Some(&PublicKeyUse::Signature)
        || key.common.key_operations.is_some()
    {
        return Err(AuthError::Identity);
    }
    let decoding = DecodingKey::from_jwk(key).map_err(|_| AuthError::Identity)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.required_spec_claims.clear();
    let claims = decode::<IdClaims>(token, &decoding, &validation)
        .map_err(|_| AuthError::Identity)?
        .claims;
    if claims.iss != issuer
        || !claims.aud.includes(client_id)
        || claims.sub.is_empty()
        || claims.sub.len() > 256
        || claims.sub.bytes().any(|b| b < 32 || b == 127)
        || claims.exp <= now
        || claims.nbf.is_some_and(|v| v > now)
        || claims.iat.is_some_and(|v| v > now.saturating_add(60))
        || nonce.is_some_and(|v| claims.nonce.as_deref() != Some(v))
    {
        return Err(AuthError::Identity);
    }
    if claims
        .email
        .as_ref()
        .is_some_and(|v| v.len() > 320 || v.bytes().any(|b| b < 32 || b == 127))
    {
        return Err(AuthError::Identity);
    }
    Ok(Identity {
        sub: claims.sub,
        email: claims.email,
    })
}
