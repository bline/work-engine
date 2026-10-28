use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use lifecycle_runtime::{NativeChildExit, NativeStdio, NativeStdioError};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use tokio::process::Command;

use crate::isolation::{self, AccessIsolationConfig, IsolatedNative, ProxyAdmission};
use crate::{NATIVE_BINARY_SHA256, NativeEvent, NativeEventKind, NativeProtocolError, parse_event};

#[derive(Debug, Error)]
pub enum NativeHarnessError {
    #[error("installed native binary does not match the bound profile pin")]
    BinaryMismatch,
    #[error("native transport failed: {0}")]
    Transport(#[from] NativeStdioError),
    #[error("native protocol failed: {0}")]
    Protocol(#[from] NativeProtocolError),
    #[error("native request serialization failed")]
    Serialization,
    #[error("native transport reached EOF")]
    Eof,
    #[error("required runtime provider credential is unavailable")]
    CredentialUnavailable,
    #[error("selected ChatGPT access lease is unavailable")]
    ChatgptLeaseUnavailable,
    #[error("trusted ChatGPT HTTPS proxy is invalid for this native route")]
    InvalidTrustedProxy,
    #[error("selected native access isolation is unavailable")]
    IsolationUnavailable,
    #[error("selected native access isolation is unavailable: {0:?}")]
    IsolationFailure(crate::IsolationDiagnostic),
    #[error("native isolated transport launch failed: {0:?}")]
    IsolationLaunchFailure(crate::IsolationDiagnostic),
    #[error("native child or relay closure was not observed after isolation failure: {0:?}")]
    CleanupUnobservedIsolation(crate::IsolationDiagnostic),
    #[error("native child or relay closure was not observed")]
    CleanupUnobserved,
}

impl NativeHarnessError {
    pub fn isolation_diagnostic(&self) -> Option<crate::IsolationDiagnostic> {
        match self {
            Self::IsolationFailure(d)
            | Self::IsolationLaunchFailure(d)
            | Self::CleanupUnobservedIsolation(d) => Some(*d),
            _ => None,
        }
    }

    pub(crate) fn isolation(reason: crate::IsolationReason) -> Self {
        Self::IsolationFailure(crate::IsolationDiagnostic {
            phase: crate::IsolationPhase::BeforeLaunch,
            reason,
        })
    }

    pub(crate) fn after_spawn(self) -> Self {
        match self {
            Self::IsolationFailure(mut d) => {
                d.phase = crate::IsolationPhase::AfterSpawn;
                Self::IsolationFailure(d)
            }
            other => other,
        }
    }

    pub(crate) fn with_cleanup(self, cleanup_observed: bool) -> Self {
        if cleanup_observed {
            self
        } else {
            match self.isolation_diagnostic() {
                Some(d) => Self::CleanupUnobservedIsolation(d),
                None => Self::CleanupUnobserved,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeAuthMode {
    None,
    RuntimeApiKey,
    ExternalChatgptAccess,
    #[cfg(feature = "synthetic-access-test")]
    SyntheticChatgptAccessTest,
}

impl NativeAuthMode {
    pub fn is_access_lease(self) -> bool {
        match self {
            Self::ExternalChatgptAccess => true,
            #[cfg(feature = "synthetic-access-test")]
            Self::SyntheticChatgptAccessTest => true,
            _ => false,
        }
    }

    pub fn is_synthetic_test(self) -> bool {
        #[cfg(feature = "synthetic-access-test")]
        {
            self == Self::SyntheticChatgptAccessTest
        }
        #[cfg(not(feature = "synthetic-access-test"))]
        {
            false
        }
    }

    fn proxy_admission(self) -> Option<ProxyAdmission> {
        match self {
            Self::ExternalChatgptAccess => Some(ProxyAdmission::Real),
            #[cfg(feature = "synthetic-access-test")]
            Self::SyntheticChatgptAccessTest => Some(ProxyAdmission::SyntheticTest),
            _ => None,
        }
    }
}

/// Access-only material. Never derive Debug or Serialize: callers must send it
/// through one private native RPC and retain only the nonsecret expiry.
pub struct ExternalChatgptLease {
    access_token: String,
    account_id: String,
    plan_type: String,
    expires_at_unix: u64,
    source_path: PathBuf,
    source_digest: [u8; 32],
}

impl ExternalChatgptLease {
    pub fn expires_at_unix(&self) -> u64 {
        self.expires_at_unix
    }

    pub fn valid_for(&self, seconds: u64) -> bool {
        lease_valid_for(self.expires_at_unix, seconds)
    }
}

/// One native child and one protocol loop. Only trusted service composition
/// receives this type; it has no entry authority or store state of its own.
pub struct NativeHarness {
    pipe: NativeStdio,
    transport_session: String,
    next_rpc: u64,
    lease_expires_at_unix: Option<u64>,
    lease_source: Option<(PathBuf, [u8; 32])>,
    lease_admitted: bool,
    auth_mode: NativeAuthMode,
    codex_home: PathBuf,
    isolated: Option<IsolatedNative>,
}

impl NativeHarness {
    pub fn spawn(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
    ) -> Result<Self, NativeHarnessError> {
        Self::spawn_with_runtime_key(executable, work, home, codex_home, transport_session, false)
    }

    /// The selected host retains the credential in its own environment and
    /// forwards it only to this owned child. It is never placed in argv or a
    /// generated config file; the child still starts from env_clear.
    pub fn spawn_with_runtime_key(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
        requires_runtime_key: bool,
    ) -> Result<Self, NativeHarnessError> {
        Self::spawn_with_auth(
            executable,
            work,
            home,
            codex_home,
            transport_session,
            if requires_runtime_key {
                NativeAuthMode::RuntimeApiKey
            } else {
                NativeAuthMode::None
            },
        )
    }

    pub fn spawn_with_auth(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
        auth_mode: NativeAuthMode,
    ) -> Result<Self, NativeHarnessError> {
        Self::spawn_with_auth_and_proxy(
            executable,
            work,
            home,
            codex_home,
            transport_session,
            auth_mode,
            None,
        )
    }

    /// The optional CONNECT proxy is a trusted route binding, never inherited
    /// from the parent environment. Only the external ChatGPT lease may use it.
    pub fn spawn_with_auth_and_proxy(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
        auth_mode: NativeAuthMode,
        https_proxy: Option<&str>,
    ) -> Result<Self, NativeHarnessError> {
        Self::spawn_with_access_isolation(
            executable,
            work,
            home,
            codex_home,
            transport_session,
            auth_mode,
            https_proxy,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_with_access_isolation(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
        auth_mode: NativeAuthMode,
        https_proxy: Option<&str>,
        isolation: Option<&AccessIsolationConfig>,
    ) -> Result<Self, NativeHarnessError> {
        if https_proxy.is_some_and(|proxy| {
            !auth_mode.is_access_lease() || !trusted_loopback_https_proxy(proxy)
        }) {
            return Err(NativeHarnessError::InvalidTrustedProxy);
        }
        if isolation.is_some() && https_proxy.is_none() {
            return Err(NativeHarnessError::InvalidTrustedProxy);
        }
        let bytes = std::fs::read(executable).map_err(|_| NativeHarnessError::BinaryMismatch)?;
        if format!("{:x}", Sha256::digest(bytes)) != NATIVE_BINARY_SHA256
            || transport_session.is_empty()
        {
            return Err(NativeHarnessError::BinaryMismatch);
        }
        if isolation.is_some() && !auth_mode.is_access_lease() {
            return Err(NativeHarnessError::isolation(
                crate::IsolationReason::TransportAuth,
            ));
        }
        // Unit fixtures may exercise the native RPC boundary directly. The
        // normal library, including feature-enabled synthetic products, never
        // starts an access lease outside its selected isolation profile.
        if auth_mode.is_access_lease() && isolation.is_none() && !cfg!(test) {
            return Err(NativeHarnessError::isolation(
                crate::IsolationReason::TransportProfile,
            ));
        }
        let mut command = Command::new(executable);
        let runtime_key = if auth_mode == NativeAuthMode::RuntimeApiKey {
            let key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| NativeHarnessError::CredentialUnavailable)?;
            if key.len() > 512 || key.is_empty() || !key.bytes().all(|b| (0x21..=0x7e).contains(&b))
            {
                return Err(NativeHarnessError::CredentialUnavailable);
            }
            Some(key)
        } else {
            None
        };
        let (tmpdir, target) = if isolation.is_some() {
            let root = codex_home.parent().ok_or_else(|| {
                NativeHarnessError::isolation(crate::IsolationReason::TransportRoot)
            })?;
            (
                root.join("tmp").into_os_string(),
                root.join("target").into_os_string(),
            )
        } else {
            (
                std::env::var_os("TMPDIR").ok_or(NativeHarnessError::BinaryMismatch)?,
                std::env::var_os("CARGO_TARGET_DIR").ok_or(NativeHarnessError::BinaryMismatch)?,
            )
        };
        command
            .args(["app-server", "--strict-config", "--listen", "stdio://"])
            .env_clear()
            .env("HOME", home)
            .env("CODEX_HOME", codex_home)
            .env("PATH", "/usr/bin:/bin")
            .env("RUST_LOG", "error")
            .env("TMPDIR", tmpdir)
            .env("CARGO_TARGET_DIR", target)
            .current_dir(work);
        if let Some(key) = runtime_key {
            command.env("OPENAI_API_KEY", key);
        }
        if let Some(proxy) = https_proxy {
            command.env("HTTPS_PROXY", proxy).env("NO_PROXY", "");
        }
        let (pipe, isolated) = if let Some(selected) = isolation {
            let proxy = https_proxy.ok_or(NativeHarnessError::InvalidTrustedProxy)?;
            let root = codex_home.parent().ok_or_else(|| {
                NativeHarnessError::isolation(crate::IsolationReason::TransportRoot)
            })?;
            let admission = auth_mode.proxy_admission().ok_or_else(|| {
                NativeHarnessError::isolation(crate::IsolationReason::TransportAdmission)
            })?;
            let (pipe, observed) = isolation::spawn(
                executable, root, work, home, codex_home, proxy, selected, admission,
            )?;
            (pipe, Some(observed))
        } else {
            (NativeStdio::spawn(&mut command, 1_048_576, 4096)?, None)
        };
        Ok(Self {
            pipe,
            transport_session,
            next_rpc: 1,
            lease_expires_at_unix: None,
            lease_source: None,
            lease_admitted: false,
            auth_mode,
            codex_home: codex_home.to_path_buf(),
            isolated,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.isolated
            .as_ref()
            .map(|observed| observed.native_pid)
            .or_else(|| self.pipe.pid())
    }

    /// This credential-bearing request is deliberately kept out of the event
    /// ingress path. The caller must discard the matching raw response too.
    pub async fn send_external_chatgpt_login(
        &mut self,
        lease: ExternalChatgptLease,
    ) -> Result<u64, NativeHarnessError> {
        if !lease.valid_for(60)
            || self
                .isolated
                .as_ref()
                .is_some_and(|selected| !selected.route_current())
        {
            return Err(NativeHarnessError::ChatgptLeaseUnavailable);
        }
        let id = self
            .send_request(
                "account/login/start",
                json!({
                    "type":"chatgptAuthTokens", "accessToken":lease.access_token,
                    "chatgptAccountId":lease.account_id, "chatgptPlanType":lease.plan_type,
                }),
            )
            .await?;
        self.lease_expires_at_unix = Some(lease.expires_at_unix);
        self.lease_source = Some((lease.source_path, lease.source_digest));
        Ok(id)
    }

    pub fn lease_valid_for(&self, seconds: u64) -> bool {
        self.lease_expires_at_unix
            .is_some_and(|expiry| lease_valid_for(expiry, seconds))
            && self
                .lease_source
                .as_ref()
                .is_some_and(|(path, digest)| source_unchanged(path, digest))
            && self
                .isolated
                .as_ref()
                .is_none_or(|selected| selected.route_current())
    }

    pub fn mark_lease_admitted(&mut self) -> Result<(), NativeHarnessError> {
        if !self.lease_valid_for(60) || self.codex_home.join("auth.json").exists() {
            return Err(NativeHarnessError::ChatgptLeaseUnavailable);
        }
        self.lease_admitted = true;
        Ok(())
    }

    pub fn entry_allowed(&self) -> bool {
        !self.auth_mode.is_access_lease()
            || (self.lease_admitted
                && self.lease_valid_for(60)
                && !self.codex_home.join("auth.json").exists())
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<u64, NativeHarnessError> {
        if self.auth_mode.is_access_lease()
            && matches!(method, "thread/start" | "turn/start")
            && !self.entry_allowed()
        {
            return Err(NativeHarnessError::ChatgptLeaseUnavailable);
        }
        let id = self.next_rpc;
        self.next_rpc = self
            .next_rpc
            .checked_add(1)
            .ok_or(NativeHarnessError::Serialization)?;
        let raw = serde_json::to_vec(&json!({"id":id,"method":method,"params":params}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(id)
    }

    pub async fn send_notification(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(&json!({"method":method,"params":params}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn send_result(&mut self, id: u64, result: Value) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(&json!({"id":id,"result":result}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn deny_request(&mut self, id: u64) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(
            &json!({"id":id,"error":{"code":-32601,"message":"unsupported native request"}}),
        )
        .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<NativeEvent, NativeHarnessError> {
        loop {
            if self.lease_admitted && !self.lease_valid_for(0) {
                return Err(NativeHarnessError::ChatgptLeaseUnavailable);
            }
            let (sequence, raw) = self.pipe.receive().await?.ok_or(NativeHarnessError::Eof)?;
            let event = match parse_event(self.transport_session.clone(), sequence, raw.clone()) {
                Ok(event) => event,
                Err(_) => NativeEvent {
                    transport_session: self.transport_session.clone(),
                    sequence,
                    sha256: format!("{:x}", Sha256::digest(&raw)),
                    raw,
                    value: Value::Null,
                    kind: NativeEventKind::Malformed,
                },
            };
            if self.lease_admitted && !self.lease_valid_for(0) {
                return Err(NativeHarnessError::ChatgptLeaseUnavailable);
            }
            match &event.kind {
                NativeEventKind::ServerRequest { id, method }
                    if method == "account/chatgptAuthTokens/refresh" =>
                {
                    self.deny_request(*id).await?;
                    return Err(NativeHarnessError::ChatgptLeaseUnavailable);
                }
                NativeEventKind::Notification { method }
                    if method == "account/updated" || method == "account/login/completed" =>
                {
                    if self.lease_admitted {
                        return Err(NativeHarnessError::ChatgptLeaseUnavailable);
                    }
                    continue;
                }
                _ => return Ok(event),
            }
        }
    }

    pub async fn terminate_and_reap(&mut self) -> Result<NativeChildExit, NativeHarnessError> {
        if let Some(observed) = self.isolated.as_mut() {
            return observed.terminate_and_reap(&mut self.pipe).await;
        }
        let exit = self.pipe.terminate_and_reap(Duration::from_secs(5)).await?;
        Ok(exit)
    }
}

/// A fixed IPv4 loopback HTTP CONNECT endpoint. Exact spelling prevents URL
/// userinfo, DNS names, paths, queries, fragments and ambiguous port forms.
pub fn trusted_loopback_https_proxy(value: &str) -> bool {
    let Some(port_text) = value.strip_prefix("http://127.0.0.1:") else {
        return false;
    };
    let Ok(port) = port_text.parse::<u16>() else {
        return false;
    };
    port != 0 && port_text == port.to_string()
}

fn lease_valid_for(expires_at_unix: u64, seconds: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    now.and_then(|now| now.checked_add(seconds))
        .is_some_and(|minimum| expires_at_unix > minimum)
}

fn source_unchanged(path: &Path, digest: &[u8; 32]) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_file()
        || meta.len() > 64 * 1024
        || meta.permissions().mode() & 0o077 != 0
    {
        return false;
    }
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(opened) = file.metadata() else {
        return false;
    };
    if opened.ino() != meta.ino() || opened.dev() != meta.dev() {
        return false;
    }
    use std::io::Read as _;
    let mut bytes = Vec::new();
    if file.take(64 * 1024 + 1).read_to_end(&mut bytes).is_err() {
        return false;
    }
    bytes.len() <= 64 * 1024 && Sha256::digest(&bytes).as_slice() == digest
}

fn jwt_payload(token: &str) -> Result<Value, NativeHarnessError> {
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(NativeHarnessError::ChatgptLeaseUnavailable);
    };
    if header.is_empty() || payload.is_empty() || signature.is_empty() || token.len() > 16 * 1024 {
        return Err(NativeHarnessError::ChatgptLeaseUnavailable);
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    serde_json::from_slice(&bytes).map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)
}

/// Read only access-only fields from a trusted, explicitly selected file store.
/// No source path, refresh token, ID token, or API key is sent to the child.
pub fn load_external_chatgpt_lease(
    source_home: &Path,
    expected_account_id: &str,
) -> Result<ExternalChatgptLease, NativeHarnessError> {
    let fail = NativeHarnessError::ChatgptLeaseUnavailable;
    if !source_home.is_absolute()
        || expected_account_id.is_empty()
        || expected_account_id.len() > 256
        || expected_account_id.trim() != expected_account_id
    {
        return Err(fail);
    }
    let home_meta = std::fs::symlink_metadata(source_home)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if !home_meta.is_dir()
        || home_meta.permissions().mode() & 0o077 != 0
        || home_meta.uid()
            != std::fs::metadata("/proc/self")
                .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?
                .uid()
    {
        return Err(fail);
    }
    // Fail closed unless this source declares the supported file backend.
    // Implicit defaults, keyring, auto and ephemeral are not interchangeable
    // with an auth.json that happens to exist.
    let config_path = source_home.join("config.toml");
    let config_meta = std::fs::symlink_metadata(&config_path)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if !config_meta.file_type().is_file()
        || config_meta.uid() != home_meta.uid()
        || config_meta.len() > 64 * 1024
    {
        return Err(fail);
    }
    let config = std::fs::read_to_string(config_path)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    let parsed: toml::Table =
        toml::from_str(&config).map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if parsed
        .get("cli_auth_credentials_store")
        .and_then(toml::Value::as_str)
        != Some("file")
    {
        return Err(fail);
    }
    let source = source_home.join("auth.json");
    let before = std::fs::symlink_metadata(&source)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if !before.file_type().is_file()
        || before.uid() != home_meta.uid()
        || before.permissions().mode() & 0o077 != 0
        || before.len() > 64 * 1024
    {
        return Err(fail);
    }
    let file =
        std::fs::File::open(&source).map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    let opened = file
        .metadata()
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if opened.ino() != before.ino() || opened.dev() != before.dev() {
        return Err(fail);
    }
    use std::io::Read as _;
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if bytes.len() > 64 * 1024 {
        return Err(fail);
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| NativeHarnessError::ChatgptLeaseUnavailable)?;
    if value["auth_mode"] != "chatgpt" || !value["OPENAI_API_KEY"].is_null() {
        return Err(fail);
    }
    let tokens = &value["tokens"];
    // A durable native login must remain the sole refresh owner. The value is
    // checked in place, never copied into the lease or child request.
    if tokens["refresh_token"]
        .as_str()
        .is_none_or(|token| token.is_empty())
    {
        return Err(fail);
    }
    let account_id = tokens["account_id"]
        .as_str()
        .ok_or(NativeHarnessError::ChatgptLeaseUnavailable)?;
    if account_id != expected_account_id {
        return Err(fail);
    }
    let id_token = tokens["id_token"]
        .as_str()
        .ok_or(NativeHarnessError::ChatgptLeaseUnavailable)?;
    let id_claims = jwt_payload(id_token)?;
    let auth_claims = &id_claims["https://api.openai.com/auth"];
    if auth_claims["chatgpt_account_id"] != expected_account_id {
        return Err(fail);
    }
    let plan_type = auth_claims["chatgpt_plan_type"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .ok_or(NativeHarnessError::ChatgptLeaseUnavailable)?;
    let access_token = tokens["access_token"]
        .as_str()
        .ok_or(NativeHarnessError::ChatgptLeaseUnavailable)?;
    let access_claims = jwt_payload(access_token)?;
    if access_claims["https://api.openai.com/auth"]["chatgpt_account_id"] != expected_account_id {
        return Err(fail);
    }
    let expires_at_unix = access_claims["exp"]
        .as_u64()
        .ok_or(NativeHarnessError::ChatgptLeaseUnavailable)?;
    if !lease_valid_for(expires_at_unix, 60) {
        return Err(fail);
    }
    Ok(ExternalChatgptLease {
        access_token: access_token.to_owned(),
        account_id: account_id.to_owned(),
        plan_type: plan_type.to_owned(),
        expires_at_unix,
        source_path: source,
        source_digest: Sha256::digest(&bytes).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_proxy_is_exact_loopback_and_only_for_external_access() {
        assert!(trusted_loopback_https_proxy("http://127.0.0.1:38471"));
        for invalid in [
            "",
            "http://127.0.0.1:0",
            "http://127.0.0.1:00080",
            "http://127.0.0.1:+80",
            "http://127.0.0.1:65536",
            "http://127.0.0.2:80",
            "http://localhost:80",
            "http://user:pass@127.0.0.1:80",
            "https://127.0.0.1:80",
            "http://127.0.0.1:80/",
            "http://127.0.0.1:80?x=1",
            "http://127.0.0.1:80#fragment",
            "http://[::1]:80",
        ] {
            assert!(!trusted_loopback_https_proxy(invalid), "{invalid}");
        }
        let nonexistent = Path::new("/no/native/binary");
        for mode in [NativeAuthMode::None, NativeAuthMode::RuntimeApiKey] {
            assert!(matches!(
                NativeHarness::spawn_with_auth_and_proxy(
                    nonexistent,
                    Path::new("/"),
                    Path::new("/"),
                    Path::new("/"),
                    "session".into(),
                    mode,
                    Some("http://127.0.0.1:38471")
                ),
                Err(NativeHarnessError::InvalidTrustedProxy)
            ));
        }
        assert!(matches!(
            NativeHarness::spawn_with_auth_and_proxy(
                nonexistent,
                Path::new("/"),
                Path::new("/"),
                Path::new("/"),
                "session".into(),
                NativeAuthMode::ExternalChatgptAccess,
                Some("http://user@127.0.0.1:80")
            ),
            Err(NativeHarnessError::InvalidTrustedProxy)
        ));
    }

    #[test]
    fn external_access_lease_rejects_absent_wrong_store_mode_account_and_expiry() {
        let temp =
            std::env::temp_dir().join(format!("work-engine-external-lease-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir(&temp).unwrap();
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::write(
            temp.join("config.toml"),
            "cli_auth_credentials_store = \"auto\"\n",
        )
        .unwrap();
        let jwt = |claims: Value| {
            format!(
                "e30.{}.sig",
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(serde_json::to_vec(&claims).unwrap())
            )
        };
        let id = jwt(
            json!({"https://api.openai.com/auth":{"chatgpt_account_id":"org-test","chatgpt_plan_type":"pro"}}),
        );
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let access = jwt(
            json!({"exp":expiry,"https://api.openai.com/auth":{"chatgpt_account_id":"org-test"}}),
        );
        let auth = json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{
            "id_token":id,"access_token":access,"refresh_token":"synthetic-refresh-never-sent","account_id":"org-test"}});
        let path = temp.join("auth.json");
        std::fs::write(&path, serde_json::to_vec(&auth).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        // Raw text scanning used to accept the decoy inside a multiline value
        // while the actual top-level store was unsupported keyring.
        std::fs::write(
            temp.join("config.toml"),
            "developer_instructions = '''\ncli_auth_credentials_store = \"file\"\n'''\n\"cli_auth_credentials_store\" = \"keyring\"\n",
        )
        .unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::write(
            temp.join("config.toml"),
            "cli_auth_credentials_store = \"file\"\ncli_auth_credentials_store = \"keyring\"\n",
        )
        .unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::write(
            temp.join("config.toml"),
            "cli_auth_credentials_store = \"file\"\n",
        )
        .unwrap();
        let parsed: toml::Table =
            toml::from_str(&std::fs::read_to_string(temp.join("config.toml")).unwrap()).unwrap();
        assert_eq!(parsed["cli_auth_credentials_store"].as_str(), Some("file"));
        assert!(load_external_chatgpt_lease(&temp, "other-org").is_err());
        let lease = load_external_chatgpt_lease(&temp, "org-test").unwrap();
        assert!(lease.valid_for(60));
        assert_eq!(lease.account_id, "org-test");
        assert_eq!(lease.plan_type, "pro");
        assert!(source_unchanged(&path, &lease.source_digest));
        std::fs::write(&path, b"synthetic-source-replaced").unwrap();
        assert!(!source_unchanged(&path, &lease.source_digest));
        std::fs::write(&path, serde_json::to_vec(&auth).unwrap()).unwrap();
        let mut api_key = auth.clone();
        api_key["OPENAI_API_KEY"] = json!("synthetic-api-key-decoy");
        std::fs::write(&path, serde_json::to_vec(&api_key).unwrap()).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        let mut wrong_access = auth.clone();
        wrong_access["tokens"]["access_token"] = json!(jwt(json!({"exp":expiry,
            "https://api.openai.com/auth":{"chatgpt_account_id":"other-org"}})));
        std::fs::write(&path, serde_json::to_vec(&wrong_access).unwrap()).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::write(&path, serde_json::to_vec(&auth).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut expired = auth;
        expired["tokens"]["access_token"] = json!(jwt(
            json!({"exp":1,"https://api.openai.com/auth":{"chatgpt_account_id":"org-test"}})
        ));
        std::fs::write(&path, serde_json::to_vec(&expired).unwrap()).unwrap();
        assert!(load_external_chatgpt_lease(&temp, "org-test").is_err());
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn wrong_binary_is_refused_before_environment_or_spawn() {
        let path = std::env::temp_dir().join(format!(
            "work-engine-native-pin-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&path, b"not the bound native executable").unwrap();
        let codex_home = path.with_extension("home");
        std::fs::create_dir(&codex_home).unwrap();
        let result = NativeHarness::spawn(
            &path,
            Path::new("/"),
            Path::new("/"),
            Path::new("/"),
            "test-session".into(),
        );
        let session_result = NativeHarness::spawn_with_auth(
            &path,
            Path::new("/"),
            Path::new("/"),
            &codex_home,
            "test-session".into(),
            NativeAuthMode::ExternalChatgptAccess,
        );
        assert!(matches!(
            session_result,
            Err(NativeHarnessError::BinaryMismatch)
        ));
        assert!(!codex_home.join("auth.json").exists());
        std::fs::remove_dir(codex_home).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(NativeHarnessError::BinaryMismatch)));
    }
}
